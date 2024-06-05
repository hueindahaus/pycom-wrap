use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request<'a> {
    pub id: u32,
    pub method: &'a str,
    pub params: Params,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Params {
    #[serde(rename_all = "camelCase")]
    InitializeParams { client_info: ClientInfo },
}

// #[derive(Deserialize, Serialize)]
// #[serde(rename_all = "camelCase")]
// pub struct InitializeParams {
//     pub client_info: ClientInfo,
// }

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}
