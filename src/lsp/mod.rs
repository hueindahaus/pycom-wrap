use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Request {
    rpc: String,
    id: u32,
    method: String,
}
