pub mod model;
pub mod client;
pub mod errors;
pub use errors::Error;

pub(crate) mod private {
    pub trait Sealed {}
}
