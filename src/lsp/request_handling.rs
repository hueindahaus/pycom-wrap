use crate::lsp::lexer::lex;
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
        return match request {
            IncommingMessage::Request { id, .. } if !self.is_active => {
                Ok(RequestHandlerAction::ResponseAction(Response {
                    id: Some(*id),
                    result: None,
                    jsonrpc: constants::JSON_RPC_VERSION,
                    error: Some(ResponseError {
                        code: ResponseErrorCode::InvalidRequest,
                        data: None,
                        message: "Server has been shut down, so new requests are invalid.",
                    }),
                }))
            }
            IncommingMessage::Request {
                ref method,
                params: Some(Params::InitializeParams { client_info, .. }),
                id,
                ..
            } if method == "initialize" => Ok(RequestHandlerAction::ResponseAction(
                self.handle_initialize_request(*id, client_info),
            )),
            IncommingMessage::Request { ref method, id, .. } if method == "shutdown" => Ok(
                RequestHandlerAction::ResponseAction(self.handle_shutdown_request(*id)),
            ),
            IncommingMessage::Request {
                id,
                ref method,
                params:
                    Some(Params::DocumentFormattingParams {
                        text_document,
                        options,
                    }),
                ..
            } if method == "textDocument/formatting" => Ok(RequestHandlerAction::ResponseAction(
                self.handle_textdocument_formatting_request(*id, text_document, options),
            )),
            IncommingMessage::Notification { ref method, .. } if method == "initialized" => {
                Ok(RequestHandlerAction::NoopAction)
            }
            IncommingMessage::Notification { method, .. } if method == "exit" => {
                Ok(RequestHandlerAction::ExitAction)
            }
            IncommingMessage::Notification { .. } => Err(format!(
                "TODO: add error types to handler so that they can be gracefully handled outside"
            )),
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
        optionts: &FormattingOptions,
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
