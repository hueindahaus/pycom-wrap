use crate::lsp::response::{Response, Result as ResponseResult, ServerCapabilities, ServerInfo};
use tracing::{error, info, warn};

use super::request::{ClientInfo, Params, Request};

pub fn handle_request<'a>(request: &'a Request<'a>) -> Result<Option<Response>, String> {
    return match request {
        Request {
            method: "initialize",
            params: Params::InitializeParams { client_info, .. },
            id,
        } => Ok(Some(handle_initialize_request(*id, client_info))),
        _ => Err("Unhandled message type".to_string()),
    };
}

pub fn handle_initialize_request(id: u32, client_info: &ClientInfo) -> Response {
    info!("Connected to: {} {}", client_info.name, client_info.version);
    return Response {
        rpc: "2.0",
        id: id,
        result: ResponseResult::InitializeResult {
            capabilities: ServerCapabilities {},
            server_info: ServerInfo {
                name: "pycom-wrapper",
                version: "0.0.0.0.0-beta",
            },
        },
    };
}
