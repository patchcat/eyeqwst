use reqwest::{header, Client, Method};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use url::Url;
use crate::model::{message::Message, user::User};

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


#[derive(Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    content: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MessageHistoryResponse {
    messages: Vec<Message>,
}

#[derive(Debug, Clone)]
pub struct Request<Path, Json, Query> {
    pub method: Method,
    pub needs_login: bool,
    pub path: Path,
    pub json: Json,
    pub query: Query,
}

impl<PathSegment, Path, Json, Query> Request<Path, Json, Query>
where PathSegment: AsRef<str>,
      Path: IntoIterator<Item = PathSegment>,
      Json: Serialize,
      Query: Serialize
{
    pub async fn invoke<Retval>(self, client: &Client, mut quaddle_url: Url, token: Option<String>) -> Result<Retval, Error>
    where Retval: DeserializeOwned {
        let mut path_segments = quaddle_url.path_segments_mut().unwrap();
        path_segments.extend(self.path);
        drop(path_segments);

        let mut req = client
            .request(self.method, quaddle_url)
            .json(&self.json)
            .query(&self.query);

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
            })
        }

        Ok(resp.json().await?)

    }
}

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

    /// Fires a request using the REST.
    pub async fn fire<PathSegment, Path, Json, Query, Retval>(
        &self,
        req: Request<Path, Json, Query>
    ) -> Result<Retval, Error>
    where PathSegment: AsRef<str>,
          Path: IntoIterator<Item = PathSegment>,
          Json: Serialize,
          Query: Serialize,
          Retval: DeserializeOwned
    {
        req.invoke(&self.client, self.quaddle_url.clone(), self.token.clone()).await
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

        let r: SignupResponse = self.fire(Request {
            method: Method::POST,
            needs_login: false,
            path: ["auth", "signup"],
            json: &SignupRequest { name, password },
            query: &()
        }).await?;

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

        let r: LoginResponse = self.fire(Request {
            method: Method::POST,
            needs_login: false,
            path: ["auth", "login"],
            json: &LoginRequest { name, password },
            query: &()
        }).await?;

        self.token = Some(r.token);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to make a client.
    fn make_client() -> Http {
        let quaddle_url = Url::parse("http://localhost:8080")
            .expect("could not parse URL");

        Http::new(quaddle_url, "quaddlecl tester".to_string())
            .expect("could not create a REST client instance")
    }

    #[tokio::test]
    async fn test_signup() {
        let http = make_client();

        let user = http.signup("meow1", "the_meower")
            .await
            .expect("signup failed");

        assert_eq!(user.name, "meow1");
    }

    #[tokio::test]
    async fn test_login() {
        let mut http = make_client();

        http.signup("meow2", "the_meower")
            .await
            .expect("signup failed");

        http.login("meow2", "the_meower")
            .await
            .expect("login failed");
    }
}
