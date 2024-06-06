use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum IncommingMessage<'a> {
    #[serde(rename_all = "camelCase")]
    Request {
        id: u32,
        method: &'a str,
        params: Params,
        jsonrpc: &'a str,
    },
    #[serde(rename_all = "camelCase")]
    Notification { method: &'a str, jsonrpc: &'a str },
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Params {
    #[serde(rename_all = "camelCase")]
    InitializeParams { client_info: ClientInfo },
    #[serde(rename_all = "camelCase")]
    InitializedParams {},
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}
