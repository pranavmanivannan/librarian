use exchanges::{
    binance_exchange::BinanceExchange, bybit_exchange::ByBitExchange, exchange::Exchange,
    huobi_exchange::HuobiExchange,
};
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
mod exchanges;
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

    let binance_exchange = HuobiExchange::new();
    let (buffer, listener) = binance_exchange.build().await;

    let _ = futures::join!(buffer, listener);

    Ok(())
}
