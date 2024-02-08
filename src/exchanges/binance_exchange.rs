use std::time::Duration;

use async_trait::async_trait;
use tokio::{task::JoinHandle, time::sleep};

use crate::{
    buffer::Buffer,
    data_packet::DataPacket,
    error::SymbolError,
    listeners::{
        binance_listener::{BinanceListener, BinanceParser, BinanceSymbolHandler},
        listener::{Listener, Parser, SymbolHandler, Symbols},
    },
};

use super::exchange::Exchange;

const BINANCE_HTTP: &str = "https://fapi.binance.com/fapi/v1/depth?symbol=";

pub struct BinanceExchange {}

#[async_trait]
impl Exchange for BinanceExchange {
    type Listener = BinanceListener;
}

impl BinanceExchange {
    pub async fn build_with_http(
        exchange_name: &str,
    ) -> (
        JoinHandle<Result<(), tungstenite::Error>>,
        JoinHandle<()>,
        JoinHandle<Result<(), SymbolError>>,
    ) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let listener = BinanceListener::listen(sender.clone()).await;
        let buffer = Buffer::create_task(exchange_name, 500, receiver);

        let http_listener: tokio::task::JoinHandle<Result<(), SymbolError>> =
            tokio::spawn(async move {
                loop {
                    let symbols = BinanceSymbolHandler::get_symbols().await;
                    if let Ok(Symbols::SymbolVector(symbols)) = symbols {
                        for symbol in symbols {
                            let new_symbol = symbol.replace("@depth", "").to_uppercase();
                            let url = format!("{}{}&limit=5", BINANCE_HTTP, new_symbol);
                            println!("url: {:?}", url);
                            let response = match reqwest::get(url).await {
                                Ok(res) => res.text().await,
                                Err(err) => return Err(SymbolError::ReqwestError(err)),
                            };
                            if let Ok(response) = response {
                                if !response.contains("code") {
                                    let result_packet =
                                        BinanceParser::parse(response.as_str().into());
                                    if let Ok(DataPacket::ST(mut packet)) = result_packet {
                                        packet.symbol_pair = new_symbol;
                                        let _ = sender.send(DataPacket::ST(packet));
                                    }
                                }
                            }
                        }
                    }
                    sleep(Duration::from_secs(5)).await;
                }
            });

        (listener, buffer, http_listener)
    }
}
