use serde::{Deserialize, Serialize};

use super::{channel::ChannelId, snowflake::{extra_sf_impls, newtype_sf_impl}, user::UserId};

/// Not exposed to clients yet.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(pub u64);

newtype_sf_impl!(MessageId);
extra_sf_impls!(MessageId);

/// Represents a Quaddle message. It is rather empty for now...
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Message {
    pub author_id: UserId,
    pub channel_id: ChannelId,
    pub content: String,
}
