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
use std::io::Read;
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Error;
use tungstenite::Message;

use super::listener::parse_bids_asks;
use super::listener::Symbols;
use super::listener::{Listener, Parser, SymbolHandler};

/// The websocket url used to connect to Huobi
const HUOBI_WS: &str = "wss://api.hbdm.vn/linear-swap-ws";
/// The http url used to request all symbols on Huobi's market.
const HUOBI_SYMBOL_API: &str = "https://api.hbdm.vn/linear-swap-api/v1/swap_contract_info";

pub struct HuobiListener {}
pub struct HuobiParser {}
pub struct HuobiSymbolHandler {}

#[async_trait]
impl Listener for HuobiListener {
    type Parser = HuobiParser;
    type SymbolHandler = HuobiSymbolHandler;

    /// This function connects to the Huobi websocket and sends all symbols to the websocket individually to subscribe
    /// to each symbol for both market incremental and snapshot data.
    ///
    /// # Returns
    /// A Result containing the write and read halves of the websocket stream if the connection was successful. Else, it
    /// will return an Error.
    async fn connect() -> Result<
        (
            SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ),
        Error,
    > {
        let (socket, _) = connect_async(HUOBI_WS).await?;
        let (mut write, read) = socket.split();
        let symbols = Self::SymbolHandler::get_symbols().await;
        if let Ok(Symbols::SymbolVector(symbols)) = symbols {
            for symbol in symbols {
                let _ = write.send(Message::Text(symbol.to_string())).await;
            }
            return Ok((write, read));
        }
        log::error!("Huobi - Connection error. Symbol retrieval failed.");
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Huobi - Connection error. Symbol retrieval failed.",
        )));
    }

    fn exchange_name() -> String {
        "Huobi".to_string()
    }
}

impl Parser for HuobiParser {
    /// Parses a tungstenite Message from the Huobi websocket for data. It first decompresses the message as Huobi sends
    /// gzip compressed data. If it is not compressed, which shouldn't occur, it will simply parse it as a string.
    ///
    /// # Arguments
    /// * `message` - A message of the `Message` type
    ///
    /// # Returns
    /// A Result containing a `DataPacket` if parsing was successful. Else, it returns a `ParseError`.
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

        if input_data.is_null() {
            Err(ParseError::ParsingError)
        } else if let Some(ping_key) = input_data.get("ping") {
            // handles pings
            let pong = json!({ "pong": ping_key }).to_string();
            Ok(DataPacket::Ping(pong))
        } else if let Some(parsed_data) = &input_data.get("tick") {
            let data_type = parsed_data["event"]
                .as_str()
                .ok_or(ParseError::ParsingError)?;

            let symbol_pair = parsed_data["ch"]
                .as_str()
                .ok_or(ParseError::ParsingError)?
                .to_uppercase();
            let asks = parse_bids_asks(
                parsed_data["asks"]
                    .as_array()
                    .ok_or(ParseError::ParsingError)?,
            );
            let bids = parse_bids_asks(
                parsed_data["bids"]
                    .as_array()
                    .ok_or(ParseError::ParsingError)?,
            );
            let cur_seq = parsed_data["version"]
                .as_i64()
                .ok_or(ParseError::ParsingError)?;
            let prev_seq = 0;
            let timestamp = parsed_data["ts"].as_i64().ok_or(ParseError::ParsingError)?;

            // Datapacket creation
            match data_type {
                "update" => {
                    let enum_creator = MarketIncremental {
                        symbol_pair,
                        asks,
                        bids,
                        cur_seq,
                        prev_seq,
                        timestamp,
                    };

                    return Ok(DataPacket::MI(enum_creator));
                }
                _ => {
                    let enum_creator = Snapshot {
                        symbol_pair,
                        asks,
                        bids,
                        cur_seq,
                        prev_seq,
                        timestamp,
                    };

                    return Ok(DataPacket::ST(enum_creator));
                }
            }
        } else {
            Err(ParseError::ParsingError)
        }
    }
}

impl SymbolHandler for HuobiSymbolHandler {
    /// Requests all tradeable symbols from Huobi's http endpoint and parses the response. It then creates a vector of
    /// strings that contain the both the market incremental and snapshot subscriptions, both of dpeth 20, for each
    /// symbol.
    ///
    /// # Returns
    /// A result containing a `Symbols` enum if the response is valid and contains the necessary symbol data. If the
    /// response or parsing is unsuccessful, it will return a `SymbolError`.
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
                "sub": format!("market.{}.depth.size_20.high_freq", symbol.to_lowercase()),
                "data_type": "incremental",
                "id": format!("id_{}", symbol)
            })
            .to_string();
            subscriptions.push(inc_subscription);

            // snapshot subscription
            let snap_subscription = json!({
                "sub": format!("market.{}.depth.size_20.high_freq", symbol.to_lowercase()),
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
