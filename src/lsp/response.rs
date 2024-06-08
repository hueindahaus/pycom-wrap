use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<'a> {
    pub id: Option<u32>,
    pub jsonrpc: &'a str,
    pub result: Option<Result>,
    pub error: Option<ResponseError<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseError<'a> {
    pub code: ResponseErrorCode,
    pub message: &'a str,
    pub data: Option<ResponseErrorData<'a>>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ResponseErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    ServerNotInitialized = -32002,
    UnknownErrorCode = -32001,
    RequestFailed = -32803,
    ServerCancelled = -32802,
    ContentModified = -32801,
    RequestCancelled = -32800,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ResponseErrorData<'a> {
    String(&'a str),
    Number(i32), // Maybe change this based on whatever this is intended to be represented as
    Bool(bool),
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub name: &'static str,
    pub version: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    pub document_formatting_provider: bool,
}
