use serde::{Deserialize, Serialize};

use super::snowflake::{extra_sf_impls, newtype_sf_impl};

/// A Quaddle channel ID.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelId(u64);

newtype_sf_impl!(ChannelId);
extra_sf_impls!(ChannelId);
