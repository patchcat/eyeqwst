use crate::model::{
    channel::ChannelId,
    message::{Message, MessageId},
    user::User,
};
use reqwest::{header, Client, Method};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("initialization error")]
    InitializationError(#[source] reqwest::Error),
    #[error("invalid Quaddle URL: {0}")]
    InvalidUrl(Url),
    #[error("reqwest error")]
    ReqwestError(#[from] reqwest::Error),
    #[error("API error: {reason} (HTTP status: {status})")]
    ApiError {
        reason: String,
        status: reqwest::StatusCode,
    },
    #[error("authorization needed")]
    AuthorizationNeeded,
}

#[derive(Clone, Serialize, Deserialize)]
struct ApiErrorResponse {
    reason: String,
}

#[derive(Debug, Clone)]
pub struct Request<Path, Json, Query> {
    pub method: Method,
    pub needs_login: bool,
    pub path: Path,
    pub json: Option<Json>,
    pub query: Query,
}

impl<PathSegment, Path, Json, Query> Request<Path, Json, Query>
where
    PathSegment: AsRef<str>,
    Path: IntoIterator<Item = PathSegment>,
    Json: Serialize,
    Query: Serialize,
{
    pub async fn invoke<Retval>(
        self,
        client: &Client,
        mut quaddle_url: Url,
        token: Option<String>,
    ) -> Result<Retval, Error>
    where
        Retval: DeserializeOwned,
    {
        let mut path_segments = quaddle_url.path_segments_mut().unwrap();
        path_segments.extend(self.path);
        drop(path_segments);

        let mut req = client.request(self.method, quaddle_url).query(&self.query);

        if let Some(json) = self.json {
            req = req.json(&json);
        }

        if self.needs_login {
            match token {
                Some(tok) => req = req.header(header::AUTHORIZATION, tok),
                None => return Err(Error::AuthorizationNeeded),
            }
        }

        let resp = req.send().await?;

        let status = resp.status();
        if !status.is_success() {
            let errresp: ApiErrorResponse = resp.json().await?;
            return Err(Error::ApiError {
                reason: errresp.reason,
                status,
            });
        }

        Ok(resp.json().await?)
    }
}

#[derive(Debug)]
pub struct Http {
    client: reqwest::Client,
    quaddle_url: Url,
    token: Option<String>,
}

impl Http {
    /// Constructs a new REST client.
    pub fn new(quaddle_url: Url, user_agent: String) -> Result<Self, Error> {
        if quaddle_url.cannot_be_a_base() {
            return Err(Error::InvalidUrl(quaddle_url));
        }

        Ok(Self {
            client: Client::builder()
                .user_agent(user_agent)
                .build()
                .map_err(Error::InitializationError)?,
            quaddle_url,
            token: None,
        })
    }

    /// Returns the token, if logged in.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Fires a request using the REST.
    pub async fn fire<PathSegment, Path, Json, Query, Retval>(
        &self,
        req: Request<Path, Json, Query>,
    ) -> Result<Retval, Error>
    where
        PathSegment: AsRef<str>,
        Path: IntoIterator<Item = PathSegment>,
        Json: Serialize,
        Query: Serialize,
        Retval: DeserializeOwned,
    {
        req.invoke(&self.client, self.quaddle_url.clone(), self.token.clone())
            .await
    }

    /// Creates an account and returns the resulting user.
    pub async fn signup(&self, name: &str, password: &str) -> Result<User, Error> {
        #[derive(Serialize)]
        struct SignupRequest<'a> {
            name: &'a str,
            password: &'a str,
        }

        #[derive(Deserialize)]
        struct SignupResponse {
            new_user: User,
        }

        let r: SignupResponse = self
            .fire(Request {
                method: Method::POST,
                needs_login: false,
                path: ["auth", "signup"],
                json: Some(SignupRequest { name, password }),
                query: &(),
            })
            .await?;

        Ok(r.new_user)
    }

    /// Logs in and authorizes the current client.
    pub async fn login(&mut self, name: &str, password: &str) -> Result<(), Error> {
        #[derive(Serialize)]
        struct LoginRequest<'a> {
            name: &'a str,
            password: &'a str,
        }

        #[derive(Deserialize)]
        struct LoginResponse {
            token: String,
        }

        let r: LoginResponse = self
            .fire(Request {
                method: Method::POST,
                needs_login: false,
                path: ["auth", "login"],
                json: Some(LoginRequest { name, password }),
                query: &(),
            })
            .await?;

        self.set_token(r.token);

        Ok(())
    }

    /// Logs out.
    pub fn logout(&mut self) {
        self.token = None;
    }

    /// Sets the token.
    pub fn set_token(&mut self, tok: String) {
        self.token = Some(tok);
    }

    /// Fetches a message.
    pub async fn fetch_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<Message, Error> {
        self.fire(Request {
            method: Method::GET,
            needs_login: false,
            path: [
                "channels",
                &channel_id.to_string(),
                "messages",
                &message_id.to_string(),
            ],
            json: None::<()>,
            query: (),
        })
        .await
    }

    /// Creates a message.
    pub async fn create_message(
        &self,
        channel_id: ChannelId,
        content: &str,
    ) -> Result<Message, Error> {
        #[derive(Serialize)]
        struct CreateMessageRequest<'a> {
            content: &'a str,
        }

        self.fire(Request {
            method: Method::POST,
            needs_login: true,
            path: ["channels", &channel_id.to_string(), "messages"],
            json: Some(CreateMessageRequest { content }),
            query: (),
        })
        .await
    }

    /// Edits a message.
    pub async fn edit_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        content: &str,
    ) -> Result<Message, Error> {
        #[derive(Serialize)]
        struct EditMessageRequest<'a> {
            content: &'a str,
        }

        self.fire(Request {
            method: Method::PATCH,
            needs_login: true,
            path: [
                "channels",
                &channel_id.to_string(),
                "messages",
                &message_id.to_string(),
            ],
            json: Some(EditMessageRequest { content }),
            query: (),
        })
        .await
    }

    /// Gets message history.
    pub async fn message_history(
        &self,
        channel_id: ChannelId,
        before: Option<MessageId>,
    ) -> Result<Vec<Message>, Error> {
        #[derive(Serialize)]
        struct MessageHistoryQuery {
            before: Option<MessageId>,
        }

        self.fire(Request {
            method: Method::GET,
            needs_login: true,
            path: ["channels", &channel_id.to_string(), "messages"],
            json: None::<()>,
            query: &MessageHistoryQuery { before },
        })
        .await
    }
}

#[cfg(test)]
pub mod tests {
    use rand::{
        distributions::{Alphanumeric, DistString},
        thread_rng,
    };
    use serial_test::serial;

    use super::*;

    /// Generates a random username.
    pub fn make_username() -> String {
        let discrim = Alphanumeric.sample_string(&mut thread_rng(), 8);

        format!("meow_{discrim}")
    }

    /// Helper function to make a client.
    pub fn make_http() -> Http {
        let quaddle_url = Url::parse("http://localhost:8080").expect("could not parse URL");

        Http::new(quaddle_url, "quaddlecl tester".to_string())
            .expect("could not create a REST client instance")
    }

    /// Helper function to make a client that's signed in to a user account.
    pub async fn make_signed_in() -> Http {
        let mut http = make_http();
        let uname = make_username();

        http.signup(&uname, "the_meower")
            .await
            .expect("failed to sign up");

        http.login(&uname, "the_meower")
            .await
            .expect("failed to log in");

        http
    }

    #[tokio::test]
    async fn test_signup() {
        let http = make_http();
        let uname = make_username();

        let user = http
            .signup(&uname, "the_meower")
            .await
            .expect("signup failed");

        assert_eq!(user.name, uname);
    }

    #[tokio::test]
    async fn test_login() {
        let mut http = make_http();
        let uname = make_username();

        http.signup(&uname, "the_meower")
            .await
            .expect("signup failed");

        http.login(&uname, "the_meower")
            .await
            .expect("login failed");

        assert_ne!(http.token(), None);
    }

    #[tokio::test]
    #[serial(message_create)]
    async fn test_fetch_message() {
        let http = make_signed_in().await;

        let msg = http
            .create_message(ChannelId(1), "meow")
            .await
            .expect("failed to create message");

        let fetched_message = http
            .fetch_message(ChannelId(1), msg.id)
            .await
            .expect("failed to fetch message");

        assert_eq!(msg.id, fetched_message.id);
        assert_eq!(msg.content, fetched_message.content);
    }

    #[tokio::test]
    #[serial(message_create)]
    async fn test_create_message() {
        let http = make_signed_in().await;

        let msg = http
            .create_message(ChannelId(1), "meow")
            .await
            .expect("failed to create message");

        assert_eq!(msg.content, "meow");
    }

    #[tokio::test]
    #[serial(message_create)]
    async fn test_edit_message() {
        let http = make_signed_in().await;

        let msg = http
            .create_message(ChannelId(1), "meow")
            .await
            .expect("failed to create message");

        let edited_message = http
            .edit_message(ChannelId(1), msg.id, "start doing this")
            .await
            .expect("failed to edit message");

        let fetched_message = http
            .fetch_message(ChannelId(1), msg.id)
            .await
            .expect("failed to fetch message");

        assert_eq!(msg.id, edited_message.id);
        assert_eq!(msg.id, fetched_message.id);
        assert_eq!("start doing this", edited_message.content);
        assert_eq!("start doing this", fetched_message.content);
    }

    #[tokio::test]
    #[serial(message_create)]
    async fn test_message_history_latest() {
        let http = make_signed_in().await;

        for content in ["meow1", "meow2"] {
            http.create_message(ChannelId(1), content)
                .await
                .expect("failed to create message");
        }

        let hist = http
            .message_history(ChannelId(1), None)
            .await
            .expect("failed to retrieve message history");

        assert_eq!(hist[0].content, "meow2");
        assert_eq!(hist[1].content, "meow1");
    }

    #[tokio::test]
    #[serial(message_create)]
    async fn test_message_history_before() {
        let http = make_signed_in().await;

        http.create_message(ChannelId(1), "meow1")
            .await
            .expect("failed to create message");

        let msg = http
            .create_message(ChannelId(1), "meow2")
            .await
            .expect("failed to create message");

        http.create_message(ChannelId(1), "meow3")
            .await
            .expect("failed to create message");

        let hist = http
            .message_history(ChannelId(1), Some(msg.id))
            .await
            .expect("failed to retrieve message history");

        assert_eq!(hist[0].content, "meow1");
    }
}
