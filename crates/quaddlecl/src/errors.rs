use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("gateway error")]
    Gateway(#[from] crate::client::gateway::Error),
    #[error("http error")]
    Http(#[from] crate::client::http::Error),
}
