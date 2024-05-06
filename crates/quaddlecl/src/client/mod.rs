use gateway::Gateway;
use http::Http;
use url::Url;

use crate::model::user::User;
use crate::Error;

/// Holds the HTTP and gateway clients.
pub struct Client {
    http: Http,
    gateway: Gateway,
}

impl Client {
    /// Creates a new Client.
    pub async fn new(quaddle_url: Url, user_agent: &str) -> Result<Self, Error> {
        Ok(Self {
            http: Http::new(quaddle_url.clone(), user_agent.to_string())?,
            gateway: Gateway::connect(quaddle_url, user_agent.to_string()).await?,
        })
    }

    /// Logs in and identifies with the gateway. Returns a (session ID, user) tuple.
    pub async fn login(&mut self, name: &str, password: &str) -> Result<(String, User), Error> {
        self.http.login(name, password).await?;
        let token = self.http.token().expect("logged in but no token set.");

        Ok(self.gateway.identify(token.to_string()).await?)
    }

    pub fn http(&self) -> &Http {
        &self.http
    }
    pub fn gateway(&self) -> &Gateway {
        &self.gateway
    }

    pub fn http_mut(&mut self) -> &mut Http {
        &mut self.http
    }

    pub fn gateway_mut(&mut self) -> &mut Gateway {
        &mut self.gateway
    }
}

impl AsRef<Http> for Client {
    fn as_ref(&self) -> &Http {
        &self.http
    }
}

impl AsMut<Http> for Client {
    fn as_mut(&mut self) -> &mut Http {
        &mut self.http
    }
}

impl AsRef<Gateway> for Client {
    fn as_ref(&self) -> &Gateway {
        &self.gateway
    }
}

impl AsMut<Gateway> for Client {
    fn as_mut(&mut self) -> &mut Gateway {
        &mut self.gateway
    }
}

pub mod gateway;
pub mod http;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_login() {
        let uname = http::tests::make_username();
        let url = Url::parse("http://localhost:8080").expect("failed to parse URL");
        let mut client = Client::new(url, "quaddlecl tester")
            .await
            .expect("failed to create client");

        client
            .http()
            .signup(&uname, "the_meower")
            .await
            .expect("failed to sign up");

        client
            .login(&uname, "the_meower")
            .await
            .expect("failed to log in");
    }
}
