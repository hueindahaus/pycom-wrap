use crate::{
    constants,
    lsp::response::{Response, Result as ResponseResult, ServerCapabilities, ServerInfo},
};
use tracing::{debug, info};

use super::{
    request::{ClientInfo, FormattingOptions, IncommingMessage, Params, TextDocumentIdentifier},
    response::{ResponseError, ResponseErrorCode},
};

pub struct RequestHandler {
    is_active: bool,
}

pub enum RequestHandlerAction<'a> {
    ResponseAction(Response<'a>),
    ExitAction,
    NoopAction,
}

impl RequestHandler {
    pub fn new() -> RequestHandler {
        return RequestHandler { is_active: true };
    }

    pub fn handle_request<'a>(
        &'a mut self,
        request: &'a IncommingMessage<'a>,
    ) -> Result<RequestHandlerAction, String> {
        if let IncommingMessage::Request { .. } = request {
            if !self.is_active {
                return Ok(RequestHandlerAction::ResponseAction(Response {
                    id: None,
                    result: None,
                    jsonrpc: constants::JSON_RPC_VERSION,
                    error: Some(ResponseError {
                        code: ResponseErrorCode::InvalidRequest,
                        data: None,
                        message: "Server has been shut down, so new requests are invalid.",
                    }),
                }));
            }
        }

        return match request {
            IncommingMessage::Request {
                method: "initialize",
                params: Some(Params::InitializeParams { client_info, .. }),
                id,
                ..
            } => Ok(RequestHandlerAction::ResponseAction(
                self.handle_initialize_request(*id, client_info),
            )),
            IncommingMessage::Request {
                method: "shutdown",
                id,
                ..
            } => Ok(RequestHandlerAction::ResponseAction(
                self.handle_shutdown_request(*id),
            )),
            IncommingMessage::Request {
                id,
                method: "textdocument/formatting",
                params:
                    Some(Params::DocumentFormattingParams {
                        text_document,
                        formatting_options,
                    }),
                ..
            } => Ok(RequestHandlerAction::ResponseAction(
                self.handle_textdocument_formatting_request(*id, text_document, formatting_options),
            )),
            IncommingMessage::Notification {
                method: "initialized",
                ..
            } => Ok(RequestHandlerAction::NoopAction),
            IncommingMessage::Notification { method: "exit", .. } => {
                return Ok(RequestHandlerAction::ExitAction)
            }
            message => Err(format!("Unhandled message type {:#?}", message)),
        };
    }

    pub fn handle_initialize_request(&self, id: u32, client_info: &ClientInfo) -> Response {
        info!("Connected to: {} {}", client_info.name, client_info.version);
        return Response {
            jsonrpc: constants::JSON_RPC_VERSION,
            id: Some(id),
            result: Some(ResponseResult::InitializeResult {
                capabilities: ServerCapabilities {
                    document_formatting_provider: true,
                },
                server_info: ServerInfo {
                    name: "pycom-wrapper",
                    version: "0.0.0.0.0.0-beta1.final",
                },
            }),
            error: None,
        };
    }

    pub fn handle_shutdown_request(&mut self, id: u32) -> Response {
        info!("Handling shutdown request");
        self.is_active = false;
        return Response {
            jsonrpc: constants::JSON_RPC_VERSION,
            id: Some(id),
            result: None,
            error: None,
        };
    }

    pub fn handle_textdocument_formatting_request(
        &self,
        id: u32,
        text_document: &TextDocumentIdentifier,
        formattion_options: &FormattingOptions,
    ) -> Response {
        info!("Handling formatting request for {}", text_document.uri);

        return Response {
            jsonrpc: constants::JSON_RPC_VERSION,
            id: Some(id),
            result: None,
            error: None,
        };
    }
}
