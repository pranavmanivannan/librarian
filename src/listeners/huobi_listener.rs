use std::io::Read;

use crate::data_packet::DataPacket;
use crate::data_packet::MarketIncremental;
use crate::data_packet::Snapshot;
use crate::error::ParseError;
use crate::error::SymbolError;
use async_trait::async_trait;
use flate2::read::GzDecoder;
use futures_util::stream::SplitSink;
use futures_util::stream::SplitStream;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::error::Error as TungsteniteError;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Error;
use tungstenite::Message;
use url::Url;

use super::listener::Symbols;
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

    async fn connect(
        websocket_url: &str,
    ) -> Result<
        (
            SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ),
        Error,
    > {
        let url_result = Url::parse(websocket_url);
        let url = match url_result {
            Ok(url) => url,
            Err(err) => {
                let error_msg = format!("URL parse error: {err}");
                return Err(TungsteniteError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error_msg,
                )));
            }
        };

        let (socket, _) = connect_async(url).await?;
        let (mut write, read) = socket.split();
        let symbols = Self::SymbolHandler::get_symbols().await;
        if let Ok(Symbols::SymbolVector(symbols)) = symbols {
            for symbol in symbols {
                let _ = write.send(Message::Text(symbol.to_string())).await;
            }
        }
        return Ok((write, read));
    }
}

impl Parser for HuobiParser {
    fn parse(message: Message) -> Result<DataPacket, ParseError> {
        let message = match message {
            Message::Text(text) => text,
            Message::Binary(bin) => {
                // Try to switch to miniz_oxide from flate2 for decompression
                let mut decoder = GzDecoder::new(&bin[..]);
                let mut decompressed = Vec::new();
                let _ = decoder.read_to_end(&mut decompressed);
                String::from_utf8(decompressed).map_err(ParseError::Utf8Error)?
            }
            _ => return Err(ParseError::ParsingError),
        };

        let message_string = message.to_string();
        let input_data: serde_json::Value =
            serde_json::from_str(&message_string).map_err(ParseError::JsonError)?;

        if !input_data.is_null() {
            if let Some(ping_key) = input_data.get("ping") {
                let pong = json!({ "pong": ping_key }).to_string();
                return Ok(DataPacket::Ping(pong));
            } else if let Some(parsed_data) = &input_data.get("tick") {
                let data_type = &parsed_data["event"];

                let symbol_pair = parsed_data["ch"]
                    .as_str()
                    .ok_or(ParseError::ParsingError)?
                    .split('.')
                    .collect::<Vec<&str>>()
                    .get(1)
                    .ok_or(ParseError::ParsingError)?
                    .to_uppercase();

                let cur_seq = parsed_data["version"]
                    .as_i64()
                    .ok_or(ParseError::ParsingError)?;
                let ts = parsed_data["ts"].as_i64().ok_or(ParseError::ParsingError)?;

                let ask_vector = parsed_data["asks"]
                    .as_array()
                    .ok_or(ParseError::ParsingError)?;
                let asks: Vec<Value> = if ask_vector.len() >= 5 {
                    ask_vector[..5].to_vec()
                } else {
                    ask_vector.to_vec()
                };

                let bid_vector = parsed_data["bids"]
                    .as_array()
                    .ok_or(ParseError::ParsingError)?;
                let bids: Vec<Value> = if bid_vector.len() >= 5 {
                    bid_vector[..5].to_vec()
                } else {
                    bid_vector.to_vec()
                };

                if data_type == "incremental" {
                    let enum_creator = MarketIncremental {
                        symbol_pair,
                        asks,
                        bids,
                        cur_seq,
                        prev_seq: 0,
                        timestamp: ts,
                    };

                    Ok(DataPacket::MI(enum_creator))
                } else {
                    let enum_creator = Snapshot {
                        symbol_pair,
                        asks,
                        bids,
                        cur_seq,
                        prev_seq: 0,
                        timestamp: ts,
                    };

                    Ok(DataPacket::ST(enum_creator))
                }
            } else {
                Err(ParseError::ParsingError)
            }
        } else {
            Err(ParseError::ParsingError)
        }
    }
}

impl SymbolHandler for HuobiSymbolHandler {
    async fn get_symbols() -> Result<Symbols, SymbolError> {
        let response = match reqwest::get(HUOBI_SYMBOL_API).await {
            Ok(res) => res,
            Err(err) => return Err(SymbolError::ReqwestError(err)),
        };

        let json_result: Value = response.json().await.map_err(SymbolError::ReqwestError)?;

        let symbol_pairs: Vec<String> = json_result["data"]
            .as_array()
            .ok_or(SymbolError::MissingSymbolsError)?
            .iter()
            .filter_map(|s| s["contract_code"].as_str())
            .map(ToString::to_string)
            .collect();

        let mut subscriptions: Vec<String> = Vec::new();
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

        Ok(Symbols::SymbolVector(subscriptions))
    }
}
