use crate::data_packet::DataPacket;
use crate::data_packet::MarketIncremental;
use crate::error::ParseError;
use crate::error::SymbolError;
use async_trait::async_trait;
use serde_json::{json, Value};
use tungstenite::Message;

use super::listener::{Listener, Parser, SymbolHandler};

const BINANCE_SYMBOL_API: &str = "https://api.binance.us/api/v3/exchangeInfo";

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
        let message_string = message.to_string();
        let input_data: serde_json::Value =
            serde_json::from_str(&message_string).map_err(ParseError::JsonError)?;

        if !input_data.is_null() && (input_data["e"] == "depthUpdate") {
            let data_type = input_data["e"].as_str().ok_or(ParseError::ParsingError)?;
            let symb_pair = input_data["s"]
                .as_str()
                .ok_or(ParseError::ParsingError)?
                .to_uppercase();

            let seq_num = input_data["u"].as_i64().ok_or(ParseError::ParsingError)?;
            let prev_seq_num = input_data["pu"].as_i64().ok_or(ParseError::ParsingError)?;
            let ts = input_data["T"].as_i64().ok_or(ParseError::ParsingError)?;

            let ask_vector = input_data["a"].as_array().ok_or(ParseError::ParsingError)?;
            let asks: Vec<Value> = if ask_vector.len() >= 5 {
                ask_vector[..5].to_vec()
            } else {
                ask_vector.to_vec()
            };

            let bid_vector = input_data["b"].as_array().ok_or(ParseError::ParsingError)?;
            let bids: Vec<Value> = if bid_vector.len() >= 5 {
                bid_vector[..5].to_vec()
            } else {
                bid_vector.to_vec()
            };

            if data_type == "depthUpdate" {
                let enum_creator = MarketIncremental {
                    symbol_pair: symb_pair,
                    asks,
                    bids,
                    cur_seq: seq_num,
                    prev_seq: prev_seq_num,
                    timestamp: ts,
                };

                return Ok(DataPacket::MI(enum_creator));
            }
            return Err(ParseError::ParsingError);
        } else {
            return Err(ParseError::ParsingError);
        }
    }
}

impl SymbolHandler for BinanceSymbolHandler {
    async fn get_symbols() -> Result<Value, SymbolError> {
        let response = match reqwest::get(BINANCE_SYMBOL_API).await {
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
