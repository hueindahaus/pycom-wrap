use crate::lsp::response::{Response, Result as ResponseResult, ServerCapabilities, ServerInfo};
use tracing::{debug, info};

use super::request::{ClientInfo, IncommingMessage, Params};

pub fn handle_request<'a>(request: &'a IncommingMessage<'a>) -> Result<Option<Response>, String> {
    return match request {
        IncommingMessage::Request {
            method: "initialize",
            params: Params::InitializeParams { client_info, .. },
            id,
            ..
        } => Ok(Some(handle_initialize_request(*id, client_info))),
        IncommingMessage::Notification {
            method: "initialized",
            ..
        } => Ok(None),
        _ => Err("Unhandled message type".to_string()),
    };
}

pub fn handle_initialize_request(id: u32, client_info: &ClientInfo) -> Response {
    info!("Connected to: {} {}", client_info.name, client_info.version);
    return Response {
        jsonrpc: "2.0",
        id: id,
        result: ResponseResult::InitializeResult {
            capabilities: ServerCapabilities {},
            server_info: ServerInfo {
                name: "pycom-wrapper",
                version: "0.0.0.0.0.0-beta1.final",
            },
        },
    };
}
