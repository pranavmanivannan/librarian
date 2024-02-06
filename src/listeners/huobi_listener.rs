use crate::data_packet::DataPacket;
use crate::data_packet::MarketIncremental;
use crate::data_packet::Snapshot;
use crate::error::ParseError;
use crate::error::SymbolError;
use async_trait::async_trait;
use serde_json::{json, Value};
use tungstenite::Message;

use super::listener::{Listener, Parser, SymbolHandler};

/// The http url used to request all symbols on Huobi's market.
const HUOBI_SYMBOL_API: &str = "https://api.hbdm.vn/linear-swap-api/v1/swap_contract_info";

pub struct HuobiListener {}
pub struct HuobiParser {}
pub struct HuobiSymbolHandler {}

#[async_trait]
impl Listener for HuobiListener {
    type Parser = HuobiParser;
    type SymbolHandler = HuobiSymbolHandler;
}

impl Parser for HuobiParser {
    fn parse(message: Message) -> Result<DataPacket, ParseError> {
        Err(ParseError::ParsingError)
    }
}

impl SymbolHandler for HuobiSymbolHandler {
    async fn get_symbols() -> Result<Value, SymbolError> {
        let response = match reqwest::get(HUOBI_SYMBOL_API).await {
            Ok(res) => res,
            Err(err) => return Err(SymbolError::ReqwestError(err)),
        };

        let json_result: Value = response.json().await.map_err(SymbolError::ReqwestError)?;

        let symbol_pairs: Vec<String> = json_result["data"]
            .as_array()
            .ok_or(SymbolError::MissingSymbolsError)?
            .iter()
            .filter_map(|s| s["symbol"].as_str())
            .map(ToString::to_string)
            .collect();


        let mut subscriptions: Vec<Value>;
        for symbol in symbol_pairs {

            // market incremental
            let inc_subscription = json!({
                "sub": format!("market.{}.depth.size_5.high_freq", symbol.to_lowercase()),
                "data_type": "incremental",
                "id": format!("id_{}", symbol)
            })
            .to_string();
            subscriptions.push(inc_subscription);

            // snapshot subscription
            let snap_subscription = json!({
                "sub": format!("market.{}.depth.size_5.high_freq", symbol.to_lowercase()),
                "data_type": "snapshot",
                "id": format!("id_{}", symbol)
            })
            .to_string();
            subscriptions.push(snap_subscription);
        }


        log::info!("Huobi - Successfully retrieved all symbols!");

        Ok(subscriptions)
        // try Ok(Value::Array(subscriptions)) if that crashes because of return type mismatch
    }
}
