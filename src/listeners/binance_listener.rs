use crate::data_packet::DataPacket;
use crate::data_packet::MarketIncremental;
use crate::data_packet::Snapshot;
use crate::error::ParseError;
use crate::error::SymbolError;
use async_trait::async_trait;
use serde_json::{json, Value};
use tungstenite::Message;

use super::listener::{Listener, Parser, SymbolHandler};

const Binance_SYMBOL_API: &str = "https://api.hbdm.vn/linear-swap-api/v1/swap_contract_info";

pub struct BinanceListener {}
pub struct BinanceParser {}
pub struct BinanceSymbolHandler {}

#[async_trait]
impl Listener for BinanceListener {
    type Parser = BinanceParser;
    type SymbolHandler = BinanceSymbolHandler;
}

impl Parser for BinanceParser {
    fn parse(message: Message) -> Result<DataPacket, ParseError> {
        
        Err(ParseError::ParsingError)
    }
}

impl SymbolHandler for BinanceSymbolHandler {
    async fn get_symbols() -> Result<Value, SymbolError> {
        let response = match reqwest::get(Binance_SYMBOL_API).await {
            Ok(res) => res,
            Err(err) => return Err(SymbolError::ReqwestError(err)),
        };

        let json_result: Value = response.json().await.map_err(SymbolError::ReqwestError)?;

        let symbol_pairs: Vec<String> = json_result["symbols"]
            .as_array()
            .ok_or(SymbolError::MissingSymbolsError)?
            .iter()
            .filter_map(|s| s["symbol"].as_str())
            .map(ToString::to_string)
            .collect();

        //https://binance-docs.github.io/apidocs/futures/en/#diff-book-depth-streams
        // market inc
        let mut symbol_list: Vec<String> = Vec::new();
        for symbol in symbol_pairs {
            symbol_list.push(format!("{symbol}@depth"));
        }

        log::info!("Binance - Successfully retrieved all symbols!");


        let symbols = json!({
            "op": "subscribe",
            "args": symbol_list,
        });

        Ok(symbols)
    }
}