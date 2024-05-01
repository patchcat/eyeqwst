use serde::{Deserialize, Serialize};

use super::channel::ChannelId;
use super::snowflake::{extra_sf_impls, newtype_sf_impl};
use super::user::User;

/// Not exposed to clients yet.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(pub u64);

newtype_sf_impl!(MessageId);
extra_sf_impls!(MessageId);

/// Represents a Quaddle message. It is rather empty for now...
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Message {
    pub id: MessageId,
    pub author: User,
    pub channel: ChannelId,
    pub content: String,
}
