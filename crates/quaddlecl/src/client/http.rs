use serde::{Serialize, Deserialize};
use crate::model::{message::Message, user::User};

#[derive(Clone, Serialize, Deserialize)]
pub struct SignupRequest {
    name: String,
    password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    name: String,
    password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    content: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SignupResponse {
    new_user: User,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    token: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MessageHistoryResponse {
    messages: Vec<Message>,
}
