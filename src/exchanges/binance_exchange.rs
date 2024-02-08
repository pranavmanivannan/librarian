use std::time::Duration;

use async_trait::async_trait;
use tokio::{task::{self, JoinHandle}, time::sleep};

use crate::{buffer::Buffer, error::SymbolError, listeners::{binance_listener::{BinanceListener, BinanceSymbolHandler}, listener::{Listener, SymbolHandler, Symbols}}};

use super::exchange::{Exchange};

const BINANCE_HTTP: &str = "https://fapi.binance.com/fapi/v1/depth?symbol=";

pub struct BinanceExchange {}

#[async_trait]
impl Exchange for BinanceExchange {
    type Listener = BinanceListener;
}

impl BinanceExchange {
    pub async fn build_with_http(
        exchange_name: &str,
    ) -> (JoinHandle<Result<(), tungstenite::Error>>, JoinHandle<()>, JoinHandle<Result<(), SymbolError>>) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let listener = BinanceListener::listen(sender.clone()).await;
        let buffer = Buffer::create_task(exchange_name, 500, receiver);

        let http_listener: tokio::task::JoinHandle<Result<(), SymbolError>> = tokio::spawn(async move {
            // let sender_clone = sender.clone()
            loop {
                let symbols = BinanceSymbolHandler::get_symbols().await;
                if let Ok(Symbols::SymbolVector(symbols)) = symbols {
                    for symbol in symbols {
                        let new_symbol = symbol.replace("@depth", "");
                        let url = format!("{}{}&limit=1000", BINANCE_HTTP, new_symbol.to_uppercase());
                        println!("{}", url);
                        let response = match reqwest::get(url).await {
                            Ok(res) => res.text().await,
                            Err(err) => return Err(SymbolError::ReqwestError(err)),
                        };
                        if let Ok(response) = response {
                            if !response.contains("code") {
                                println!("{}", response.as_str());
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
