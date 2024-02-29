use std::sync::Arc;
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
use background::stats_loop;
use stats::MetricManager;
use tokio::signal;
use tokio_util::sync::CancellationToken;

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

    let token = CancellationToken::new();

    let metric_manager = Arc::new(MetricManager::new());
    let bybit = ByBitExchange::build("ByBit", metric_manager.clone(), token.clone()).await;
    let huobi = HuobiExchange::build("Huobi", metric_manager.clone(), token.clone()).await;
    let binance = BinanceExchange::build("Binance", metric_manager.clone(), token.clone()).await;
    stats_loop(metric_manager).await;

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

    let _ = match signal::ctrl_c().await {
        Ok(_) => {
            token.cancel();
            println!("Shutting down...");
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    };

    Ok(())
}
