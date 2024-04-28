use serde::{Deserialize, Serialize};

use super::snowflake::{extra_sf_impls, newtype_sf_impl};

/// A Quaddle user ID.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub u64);

newtype_sf_impl!(UserId);
extra_sf_impls!(UserId);

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub name: String,
}
