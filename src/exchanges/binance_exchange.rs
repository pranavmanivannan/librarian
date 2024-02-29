use std::sync::Arc;

use async_trait::async_trait;
use futures_util::future::join_all;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::{
    buffer::Buffer,
    data_packet::DataPacket,
    error::SymbolError,
    listeners::{
        binance_listener::{BinanceListener, BinanceParser, BinanceSymbolHandler},
        listener::{Listener, Parser, SymbolHandler, Symbols},
    },
    stats::MetricManager,
};

use super::exchange::{Exchange, TaskSet};

const BINANCE_HTTP: &str = "https://fapi.binance.com/fapi/v1/depth?symbol=";

pub struct BinanceExchange {}

#[async_trait]
impl Exchange for BinanceExchange {
    type Listener = BinanceListener;

    /// An overriden `build` function that returns a `TaskSet` of the `Extended` variant.
    ///
    /// # Arguments
    /// * `exchange_name` - A string slice that holds the name of the exchange. This is used when creating the buffer
    /// and should refer to the first half of the bucket name on InfluxDB.
    ///
    /// # Returns
    /// A `TaskSet` containing a `JoinHandle` for the listener, buffer, and an additional `JoinHandle` for the HTTP
    /// listener. The HTTP listener is used to retrieve orderbook snapshots as Binance does not send them through the
    /// websocket stream.
    async fn build(
        exchange_name: &str,
        metric_manager: Arc<MetricManager>,
        cancellation_token: CancellationToken,
    ) -> TaskSet {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let listener = BinanceListener::listen(sender.clone(), metric_manager.clone()).await;
        let buffer = Buffer::create_task(
            exchange_name,
            500,
            receiver,
            metric_manager.clone(),
            cancellation_token,
        );

        let http_listener: JoinHandle<Result<(), SymbolError>> = tokio::spawn(async move {
            loop {
                // Get all Binance symbols to request snapshots for. We should be getting the symbols in the form of a
                // vector, so a string of symbols means we requested an incorrect endpoint or the get_symbols method is
                // malfunctioning.
                let mut symbols = match BinanceSymbolHandler::get_symbols().await {
                    Ok(Symbols::SymbolVector(symbols)) => symbols,
                    Ok(Symbols::SymbolString(_)) => continue,
                    Err(_e) => continue,
                };

                // Edit each symbol to its corresponding snapshot URL endpoint.
                for symbol in &mut symbols {
                    let cleaned_symbol = symbol.replace("@depth", "").to_uppercase();
                    *symbol = format!("{}{}&limit=5", BINANCE_HTTP, cleaned_symbol);
                }

                // Permanently loop over requesting the snapshot for each endpoint and parsing it. There should be no
                // reason to break out of this loop as all error handling, such as getting an invalid response,
                // is done within it.
                loop {
                    let responses = symbols.iter().map(|symbol| {
                        let sender_clone = sender.clone();
                        async move {
                            let response = match reqwest::get(symbol).await {
                                Ok(res) => res,
                                Err(_e) => return,
                            };
                            let response_text = match response.text().await {
                                Ok(res_text) => res_text,
                                Err(_e) => return,
                            };
                            if response_text.contains("code") {
                                return;
                            }
                            let packet = BinanceParser::parse(response_text.into());
                            if let Ok(DataPacket::ST(mut packet)) = packet {
                                packet.symbol_pair = symbol.to_string();
                                let _ = sender_clone.send(DataPacket::ST(packet));
                            }
                        }
                    });
                    let _ = join_all(responses).await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        });

        TaskSet::Extended(listener, buffer, http_listener)
    }
}
