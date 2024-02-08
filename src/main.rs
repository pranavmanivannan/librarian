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

    let (bybit_listener, bybit_buffer) = ByBitExchange::build("ByBit").await;
    let (huobi_listener, huobi_buffer) = HuobiExchange::build("Huobi").await;
    let (binance_listener, binance_buffer, binance_http) =
        BinanceExchange::build_with_http("Binance").await;

    let _ = futures::join!(
        bybit_listener,
        bybit_buffer,
        huobi_listener,
        huobi_buffer,
        binance_listener,
        binance_buffer,
        binance_http
    );

    Ok(())
}
