use crate::buffer::DataType;
use crate::data_packet::deserialize_packet;
use crate::data_packet::DataPacket;
use crate::error::ParseError;
use crate::error::SymbolError;
use async_trait::async_trait;
use futures_util::stream::SplitSink;
use futures_util::stream::SplitStream;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::error::Error as TungsteniteError;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Error;
use tungstenite::Message;

use super::listener::Symbols;
use super::listener::{Listener, Parser, SymbolHandler};

const BINANCE_WS: &str = "wss://fstream.binance.com";
const BINANCE_SYMBOL_API: &str = "https://api.binance.us/api/v3/exchangeInfo";

pub struct BinanceListener {}
pub struct BinanceParser {}
pub struct BinanceSymbolHandler {}

#[async_trait]
impl Listener for BinanceListener {
    type Parser = BinanceParser;
    type SymbolHandler = BinanceSymbolHandler;

    async fn connect() -> Result<
        (
            SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ),
        Error,
    > {
        let symbol_list = Self::SymbolHandler::get_symbols().await;
        if let Ok(Symbols::SymbolVector(symbol_list)) = symbol_list {
            let symbols = symbol_list.join("/");
            let binance_url = format!("{}/stream?streams={}", BINANCE_WS, symbols);
            let (socket, _) = connect_async(binance_url).await?;
            let (mut write, read) = socket.split();
            let _ = write.send(Message::Text(symbols.to_string())).await;
            return Ok((write, read));
        } else {
            return Err(TungsteniteError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Symbol Error",
            )));
        }
    }
}

impl Parser for BinanceParser {
    fn parse(message: Message) -> Result<DataPacket, ParseError> {
        let message_string = message.to_string();
        let message_data: serde_json::Value =
            serde_json::from_str(&message_string).map_err(ParseError::JsonError)?;

        if message_data.is_null() {
            return Err(ParseError::ParsingError);

        } else if message_string.contains("lastUpdateId") {
            // ST case
            let fields = ["NULL", "asks", "bids", "lastUpdateId", "NULL", "T"];
            return deserialize_packet(&message_data, &fields, DataType::ST);

        } else if let Some(parsed_data) = &message_data.get("data") {
            // MI case
            let fields = ["s", "a", "b", "u", "pu", "T"];
            return deserialize_packet(parsed_data, &fields, DataType::MI);

        } else {
            return Err(ParseError::ParsingError);
        }
    }
}

impl SymbolHandler for BinanceSymbolHandler {
    async fn get_symbols() -> Result<Symbols, SymbolError> {
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
            symbol_list.push(format!("{}@depth", symbol.to_lowercase()));
        }

        log::info!("Binance - Successfully retrieved all symbols!");

        Ok(Symbols::SymbolVector(symbol_list))
    }
}
