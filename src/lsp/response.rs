use ::serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<'a> {
    pub id: u32,
    pub jsonrpc: &'a str,
    pub result: Result,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Result {
    #[serde(rename_all = "camelCase")]
    InitializeResult {
        capabilities: ServerCapabilities,
        server_info: ServerInfo,
    },
}

// #[derive(Serialize)]
// #[serde(rename_all = "camelCase")]
// pub struct InitializeResult {
//     pub capabilities: ServerCapabilities,
//     pub server_info: ServerInfo,
// }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub name: &'static str,
    pub version: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {}
