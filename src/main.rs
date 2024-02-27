use exchanges::{
    binance_exchange::BinanceExchange,
    bybit_exchange::ByBitExchange,
    exchange::{Exchange, TaskSet},
    huobi_exchange::HuobiExchange,
};
use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};
use stats::{PACKETSIZE, PARSETIME, THROUGHPUT};
use background::stats_loop;

mod background;
mod buffer;
mod data_packet;
mod error;
mod exchanges;
mod listeners;
mod stats;
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

    let bybit = ByBitExchange::build("ByBit").await;
    let huobi = HuobiExchange::build("Huobi").await;
    let binance = BinanceExchange::build("Binance").await;
    stats_loop(&THROUGHPUT, &PARSETIME, &PACKETSIZE).await;

    for exchange in [bybit, huobi, binance] {
        match exchange {
            TaskSet::Default(listener, buffer) => {
                let _ = tokio::join!(listener, buffer);
            }
            TaskSet::Extended(listener, buffer, additional) => {
                let _ = tokio::join!(listener, buffer, additional);
            }
        }
    }

    Ok(())
}
