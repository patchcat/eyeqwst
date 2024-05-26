use serde::{Deserialize, Serialize};

use super::snowflake::{extra_sf_impls, newtype_sf_impl};

/// A Quaddle channel ID.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct ChannelId(pub u64);

newtype_sf_impl!(ChannelId);
extra_sf_impls!(ChannelId);
