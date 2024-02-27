use super::listener::{parse_bids_asks, Listener, Parser, SymbolHandler, Symbols};
use crate::{
    data_packet::{DataPacket, MarketIncremental, Snapshot},
    error::{ParseError, SymbolError},
};
use async_trait::async_trait;
use futures_util::{
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::error::Error as TungsteniteError, MaybeTlsStream, WebSocketStream,
};
use tungstenite::{Error, Message};

const BINANCE_WS: &str = "wss://fstream.binance.com";
const BINANCE_SYMBOL_API: &str = "https://api.binance.us/api/v3/exchangeInfo";

pub struct BinanceListener {}
pub struct BinanceParser {}
pub struct BinanceSymbolHandler {}

#[async_trait]
impl Listener for BinanceListener {
    type Parser = BinanceParser;
    type SymbolHandler = BinanceSymbolHandler;

    /// This function connects to the Binance websocket. Unlike separately sending a message to subscribe to each symbol
    /// and channel endpoint, Binance subscribes to all symbols by inputting the symbols and depth of the orderbook for
    /// each symbol in the URL. However, it does not receive orderbook snapshots through a websocket stream. That is
    /// done separately through a REST API.
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
        let symbol_list = Self::SymbolHandler::get_symbols().await;
        if let Ok(Symbols::SymbolVector(symbol_list)) = symbol_list {
            let symbols = symbol_list.join("/");
            let binance_url = format!("{}/stream?streams={}", BINANCE_WS, symbols);
            let (socket, _) = connect_async(binance_url).await?;
            let (write, read) = socket.split();
            return Ok((write, read));
        } else {
            return Err(TungsteniteError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Binance - Connection error. Symbol retrieval failed.",
            )));
        }
    }

    fn exchange_name() -> String {
        "Binance".to_string()
    }
}

impl Parser for BinanceParser {
    /// Parses a tungstenite Message from the Binance websocket for data.
    ///
    /// # Arguments
    /// * `message` - A message of the `Message` type
    ///
    /// # Returns
    /// A Result containing a `DataPacket` if parsing was successful, else a `ParseError`.
    fn parse(message: Message) -> Result<DataPacket, ParseError> {
        let message_string = message.to_string();
        let message_data: serde_json::Value =
            serde_json::from_str(&message_string).map_err(ParseError::JsonError)?;

        if message_data.is_null() {
            Err(ParseError::ParsingError)
        } else if message_string.contains("lastUpdateId") {
            // Snapshot case
            let symbol_pair = "NULL".to_string();
            let asks = parse_bids_asks(
                message_data["asks"]
                    .as_array()
                    .ok_or(ParseError::ParsingError)?,
            );
            let bids = parse_bids_asks(
                message_data["bids"]
                    .as_array()
                    .ok_or(ParseError::ParsingError)?,
            );
            let cur_seq = message_data["lastUpdateId"]
                .as_i64()
                .ok_or(ParseError::ParsingError)?;
            let prev_seq = 0;
            let timestamp = message_data["T"].as_i64().ok_or(ParseError::ParsingError)?;

            let enum_creator = Snapshot {
                symbol_pair,
                asks,
                bids,
                cur_seq,
                prev_seq,
                timestamp,
            };

            return Ok(DataPacket::ST(enum_creator));
        } else if let Some(parsed_data) = &message_data.get("data") {
            // Market incremental case
            let symbol_pair = parsed_data["s"]
                .as_str()
                .ok_or(ParseError::ParsingError)?
                .to_uppercase();
            let asks = parse_bids_asks(
                parsed_data["a"]
                    .as_array()
                    .ok_or(ParseError::ParsingError)?,
            );
            let bids = parse_bids_asks(
                parsed_data["b"]
                    .as_array()
                    .ok_or(ParseError::ParsingError)?,
            );
            let cur_seq = parsed_data["u"].as_i64().ok_or(ParseError::ParsingError)?;
            let prev_seq = parsed_data["pu"].as_i64().ok_or(ParseError::ParsingError)?;
            let timestamp = parsed_data["T"].as_i64().ok_or(ParseError::ParsingError)?;

            let enum_creator = MarketIncremental {
                symbol_pair,
                asks,
                bids,
                cur_seq,
                prev_seq,
                timestamp,
            };

            return Ok(DataPacket::MI(enum_creator));
        } else {
            return Err(ParseError::ParsingError);
        }
    }
}

#[async_trait]
impl SymbolHandler for BinanceSymbolHandler {
    /// Requests all tradeable symbols from the Binance http endpoint and parses the response. It then creates a vector
    /// of formated strings. The vector that is returned is then converted into a single string and used when
    /// connecting to the Binance websocket to subscribe to all symbols.
    ///
    /// # Returns
    /// A result containing a `Value` if the response is valid and contains the necessary symbol data. If the response
    /// or parsing is unsuccessful, it will return a `SymbolError`.
    async fn get_symbols() -> Result<Symbols, SymbolError> {
        let response = match reqwest::get(BINANCE_SYMBOL_API).await {
            Ok(res) => res,
            Err(e) => return Err(SymbolError::ReqwestError(e)),
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
