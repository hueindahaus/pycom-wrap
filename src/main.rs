mod constants;
mod lsp;
mod rpc;
mod scanner;
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use tracing::{event, Level};
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

    event!(Level::INFO, "Starting pycom-wrap...");
    let reader = std::io::stdin();
    let scanner = scanner::Scanner::from_reader(reader, &rpc::split_fn);

    for scan in scanner.into_iter() {
        let bytes: &[u8] = &scan;
        let message: lsp::Request = rpc::decode_message(bytes);
    }
}
