mod constants;
mod lsp;
mod rpc;
mod scanner;
use core::panic;
use std::{fs::OpenOptions, io::Write};

use lsp::{
    request_handling::{self, RequestHandler, RequestHandlerAction},
    response,
};
use tracing::{error, event, info, Level};
use tracing_subscriber::{self, layer::SubscriberExt};
const LOG_FILE_PATH: &str = "~/workspaces/pycom_wrap/log.txt";
fn main() {
    // let _ = std::fs::remove_file(LOG_FILE_PATH);

    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(LOG_FILE_PATH)
        .unwrap();
    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().compact().with_writer(file));
    let _ = tracing::subscriber::set_global_default(subscriber);
    std::panic::set_hook(Box::new(|pf| {
        error!("{}", pf.to_string());
    }));

    event!(Level::INFO, "Starting pycom-wrap...");
    let reader = std::io::stdin();
    let scanner = scanner::Scanner::from_reader(reader, &rpc::split_fn);
    let mut writer = std::io::stdout();
    let mut request_handler = RequestHandler::new();

    for scan in scanner {
        let msg = scan;

        info!("[Read] {}", std::str::from_utf8(&msg).unwrap());
        let message =
            rpc::decode_message(&msg).unwrap_or_else(|w| panic!("Error decoding message: {}", w));

        let action = request_handler
            .handle_request(&message)
            .unwrap_or_else(|w| panic!("Error handling request: {}", w));

        match action {
            RequestHandlerAction::ResponseAction(response) => {
                let encoded_message = rpc::encode_message(&response)
                    .unwrap_or_else(|w| panic!("Error encoding message: {}", w));

                info!("[Write] {}", std::str::from_utf8(&encoded_message).unwrap());

                writer
                    .write(&encoded_message)
                    .expect("Error when writing to output");
                writer.flush().expect("Error when flushing writer.")
            }
            RequestHandlerAction::ExitAction => break,
            RequestHandlerAction::NoopAction => (),
        }
    }

    info!("Exiting pycom-wrap..");
}
