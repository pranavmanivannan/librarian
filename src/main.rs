use background::stats_loop;
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
use stats::MetricManager;
use std::sync::Arc;
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

    // Create a cancellation token to use to signal the tokio tasks to shutdown.
    let token = CancellationToken::new();
    let token_clone = token.clone();

    // Spawn a tokio task to listen for a ctrl-c to cancel the cancellation token.
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(_) => {
                token.cancel();
                log::info!("Shutting down...");
            }
            Err(e) => {
                log::error!("Error: {}", e);
            }
        };
    });

    let metric_manager = Arc::new(MetricManager::new());
    let bybit = ByBitExchange::build("ByBit", metric_manager.clone(), token_clone.clone()).await;
    let huobi = HuobiExchange::build("Huobi", metric_manager.clone(), token_clone.clone()).await;
    let binance =
        BinanceExchange::build("Binance", metric_manager.clone(), token_clone.clone()).await;
    stats_loop(metric_manager, token_clone.clone()).await;

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

    // Anything past this point must wait for all tokio tasks to complete due to the tokio::join!() above.

    Ok(())
}
