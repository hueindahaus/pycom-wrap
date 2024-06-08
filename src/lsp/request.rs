use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
pub enum IncommingMessage<'a> {
    #[serde(rename_all = "camelCase")]
    Request {
        id: u32,
        method: &'a str,
        params: Option<Params>,
        jsonrpc: &'a str,
    },
    #[serde(rename_all = "camelCase")]
    Notification { method: &'a str, jsonrpc: &'a str },
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Params {
    #[serde(rename_all = "camelCase")]
    InitializeParams { client_info: ClientInfo },
    #[serde(rename_all = "camelCase")]
    InitializedParams {},
    #[serde(rename_all = "camelCase")]
    DocumentFormattingParams {
        text_document: TextDocumentIdentifier,
        formatting_options: FormattingOptions,
    },
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FormattingOptions {
    tabsize: u32,
    insert_spaces: bool,
    trim_trailing_whitespace: Option<bool>,
    insert_final_newline: Option<bool>,
    trim_final_newlines: Option<bool>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}
