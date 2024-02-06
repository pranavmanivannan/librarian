use crate::data_packet::DataPacket;
use crate::data_packet::MarketIncremental;
use crate::data_packet::Snapshot;
use crate::error::ParseError;
use crate::error::SymbolError;
use async_trait::async_trait;
use serde_json::{json, Value};
use tungstenite::Message;

use super::listener::{Listener, Parser, SymbolHandler};

/// The http url used to request all symbols on ByBit's market.
const BYBIT_SYMBOL_API: &str = "https://api-testnet.bybit.com/v5/market/tickers?category=linear";

pub struct ByBitListener {}

pub struct ByBitParser {}
pub struct ByBitSymbolHandler {}

#[async_trait]
impl Listener for ByBitListener {
    type Parser = ByBitParser;
    type SymbolHandler = ByBitSymbolHandler;
}

impl Parser for ByBitParser {
    fn parse(message: Message) -> Result<DataPacket, ParseError> {
        let message_string = message.to_string();
        let input_data: serde_json::Value =
            serde_json::from_str(&message_string).map_err(ParseError::JsonError)?;

        if !input_data.is_null()
            && (input_data["type"] == "snapshot" || input_data["type"] == "delta")
        {
            let parsed_data = &input_data["data"];
            let data_type = &input_data["type"];

            let symb_pair = parsed_data["s"]
                .as_str()
                .ok_or(ParseError::ParsingError)?
                .to_uppercase();

            let seq_num = parsed_data["u"].as_i64().ok_or(ParseError::ParsingError)?;
            let ts = input_data["ts"].as_i64().ok_or(ParseError::ParsingError)?;

            let ask_vector = parsed_data["a"]
                .as_array()
                .ok_or(ParseError::ParsingError)?;
            let asks: Vec<Value> = if ask_vector.len() >= 5 {
                ask_vector[..5].to_vec()
            } else {
                ask_vector.to_vec()
            };

            let bid_vector = parsed_data["b"]
                .as_array()
                .ok_or(ParseError::ParsingError)?;
            let bids: Vec<Value> = if bid_vector.len() >= 5 {
                bid_vector[..5].to_vec()
            } else {
                bid_vector.to_vec()
            };

            if data_type == "delta" {
                let enum_creator = MarketIncremental {
                    symbol_pair: symb_pair,
                    asks,
                    bids,
                    cur_seq: seq_num,
                    prev_seq: 0,
                    timestamp: ts,
                };

                Ok(DataPacket::MI(enum_creator))
            } else if data_type == "snapshot" {
                let enum_creator = Snapshot {
                    symbol_pair: symb_pair,
                    asks,
                    bids,
                    cur_seq: seq_num,
                    prev_seq: 0,
                    timestamp: ts,
                };

                Ok(DataPacket::ST(enum_creator))
            } else {
                Err(ParseError::ParsingError)
            }
        } else {
            Err(ParseError::ParsingError)
        }
    }
}

impl SymbolHandler for ByBitSymbolHandler {
    async fn get_symbols() -> Result<Value, SymbolError> {
        let response = match reqwest::get(BYBIT_SYMBOL_API).await {
            Ok(res) => res,
            Err(err) => return Err(SymbolError::ReqwestError(err)),
        };

        let json_result: Value = response.json().await.map_err(SymbolError::ReqwestError)?;

        let symbol_pairs: Vec<String> = json_result["result"]["list"]
            .as_array()
            .ok_or(SymbolError::MissingSymbolsError)?
            .iter()
            .filter_map(|s| s["symbol"].as_str())
            .map(ToString::to_string)
            .collect();

        let mut symbol_list: Vec<String> = Vec::new();
        for symbol in symbol_pairs {
            symbol_list.push(format!("orderbook.50.{symbol}"));
        }

        symbol_list.retain(|symbol| symbol != "orderbook.50.OKBUSDT");

        log::info!("ByBit - Successfully retrieved all symbols!");

        let symbols = json!({
            "op": "subscribe",
            "args": symbol_list,
        });

        Ok(symbols)
    }
}
