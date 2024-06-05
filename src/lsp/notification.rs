use serde::{Deserialize, Serialize};
// Notification
#[derive(Deserialize, Serialize)]
pub struct Notification {
    pub rpc: String,
    pub method: String,
}
