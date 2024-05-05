pub mod client;
pub mod errors;
pub mod model;
pub use errors::Error;

pub(crate) mod private {
    pub trait Sealed {}
}
