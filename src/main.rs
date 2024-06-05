mod constants;
mod lsp;
mod rpc;
mod scanner;
use core::panic;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

use tracing::{error, event, info, Level};
use tracing_subscriber::{self, layer::SubscriberExt};

fn main() {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("./log.txt")
        .unwrap();
    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().compact().with_writer(file));
    tracing::subscriber::set_global_default(subscriber);
    std::panic::set_hook(Box::new(|pf| {
        error!("{}", pf.to_string());
    }));

    event!(Level::INFO, "Starting pycom-wrap...");
    let reader = std::io::stdin();
    let scanner = scanner::Scanner::from_reader(reader, &rpc::split_fn);
    let mut writer = std::io::stdout();

    for scan in scanner {
        let msg = scan;
        let message = match rpc::decode_message(&msg) {
            Ok(decoded_message) => decoded_message,
            Err(e) => panic!("{}", e.to_string()),
        };

        let response_opt = match lsp::request_handler::handle_request(&message) {
            Ok(Some(response)) => Some(response),
            Ok(None) => None,
            Err(err_msg) => panic!("{}", err_msg.to_string()),
        };

        if response_opt.is_some() {
            match rpc::encode_message(&response_opt.unwrap()) {
                Ok(encoded_message) => {
                    writer.write(&encoded_message);
                    ()
                }
                Err(e) => panic!("{}", e.to_string()),
            }
        }
    }
}
