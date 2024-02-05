use background::storage_loop;
use buffer::Buffer;
use listeners::{bybit_listener::ByBitListener, listener::Listener};
use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};

mod background;
mod buffer;
mod data_packet;
mod error;
mod listeners;
// main.rs

/// Sets up a logger and corresponding log file, then spawns in listeners and buffers.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build("log/output.log")?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

    log4rs::init_config(config)?;

    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

    let bybit_listener = ByBitListener::setup().await?;
    let bybit = bybit_listener.listen(sender).await;

    tokio::spawn(storage_loop(Buffer::new("ByBit", 500), receiver));

    let _ = futures::join!(bybit);

    Ok(())
}
