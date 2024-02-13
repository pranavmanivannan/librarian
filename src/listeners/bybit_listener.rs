use crate::buffer::DataType;
use crate::data_packet;
use crate::data_packet::deserialize_packet;
use crate::data_packet::DataPacket;
use crate::error::ParseError;
use crate::error::SymbolError;
use async_trait::async_trait;
use futures_util::stream::SplitSink;
use futures_util::stream::SplitStream;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Error;
use tungstenite::Message;

use super::listener::Symbols;
use super::listener::{Listener, Parser, SymbolHandler};

/// The websocket url used to connect to ByBit's perpetuals and futures market data.
const BYBIT_WS: &str = "wss://stream.bybit.com/v5/public/linear";
/// The http url used to request all symbols on ByBit's market.
const BYBIT_SYMBOL_API: &str = "https://api-testnet.bybit.com/v5/market/tickers?category=linear";

pub struct ByBitListener {}

pub struct ByBitParser {}
pub struct ByBitSymbolHandler {}

#[async_trait]
impl Listener for ByBitListener {
    type Parser = ByBitParser;
    type SymbolHandler = ByBitSymbolHandler;

    async fn connect() -> Result<
        (
            SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ),
        Error,
    > {
        let (socket, _) = connect_async(BYBIT_WS).await?;
        let (mut write, read) = socket.split();
        let symbols = Self::SymbolHandler::get_symbols().await;
        if let Ok(Symbols::SymbolString(symbols)) = symbols {
            let _ = write.send(Message::Text(symbols)).await;
            return Ok((write, read));
        }
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Symbol Error",
        )));
    }
}

impl Parser for ByBitParser {
    fn parse(message: Message) -> Result<DataPacket, ParseError> {
        let message_string = message.to_string();
        let input_data: serde_json::Value =
            serde_json::from_str(&message_string).map_err(ParseError::JsonError)?;

        if input_data.is_null() {
            return Err(ParseError::ParsingError);

        } else if let Some(parsed_data) = &input_data.get("data") {
            // data type in input data instead of parsed data
            let data_type = input_data["type"].as_str().ok_or(ParseError::ParsingError)?;
            let dtype = match data_type {
                "snapshot" => DataType::ST,
                "delta" => DataType::MI,
                _ => Err(ParseError::ParsingError)?,
            };

            // deserialize
            let fields = ["s", "a", "b", "u", "NULL", "NULL"];
            let result_packet = deserialize_packet(parsed_data, &fields, dtype);

            // timestamp is in input data instead of json data, so it needs to be overwritten
            match result_packet {
                Ok(DataPacket::ST(mut packet)) => {
                    packet.timestamp = input_data["ts"].as_i64().ok_or(ParseError::ParsingError)?;
                    return Ok(data_packet::DataPacket::ST(packet));
                }
                Ok(DataPacket::MI(mut packet)) => {
                    packet.timestamp = input_data["ts"].as_i64().ok_or(ParseError::ParsingError)?;
                    return Ok(data_packet::DataPacket::MI(packet));
                }
                _ => return Err(ParseError::ParsingError),
            }
        } else {
            return Err(ParseError::ParsingError);
        }
    }
}

impl SymbolHandler for ByBitSymbolHandler {
    async fn get_symbols() -> Result<Symbols, SymbolError> {
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

        Ok(Symbols::SymbolString(symbols.to_string()))
    }
}
