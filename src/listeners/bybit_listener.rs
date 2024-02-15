use super::listener::{parse_bids_asks, Listener, Parser, SymbolHandler, Symbols};
use crate::{
    data_packet::{DataPacket, MarketIncremental, Snapshot},
    error::{ParseError, SymbolError},
};
use async_trait::async_trait;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::{Error, Message};

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

    /// This function connects to the ByBit websocket and sends all symbols to the websocket through a JSON subscription
    /// message. There is no need to subscribe to both market incremental and snapshot data as ByBit sends both types of
    /// data through the same channel for each symbol.
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
        let (socket, _) = connect_async(BYBIT_WS).await?;
        let (mut write, read) = socket.split();
        let symbols = Self::SymbolHandler::get_symbols().await;
        if let Ok(Symbols::SymbolString(symbols)) = symbols {
            let _ = write.send(Message::Text(symbols)).await;
            return Ok((write, read));
        }
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "ByBit - Connection error. Symbol retrieval failed.",
        )));
    }

    fn exchange_name() -> String {
        "ByBit".to_string()
    }
}

impl Parser for ByBitParser {
    /// Parses a tungstenite Message from the ByBit websocket for data.
    ///
    /// # Arguments
    /// * `message` - A message of the `Message` type
    ///
    /// # Returns
    /// A Result containing a `DataPacket` if parsing was successful, else a `ParseError`.
    fn parse(message: Message) -> Result<DataPacket, ParseError> {
        let message_string = message.to_string();
        let input_data: serde_json::Value =
            serde_json::from_str(&message_string).map_err(ParseError::JsonError)?;

        if input_data.is_null() {
            Err(ParseError::ParsingError)
        } else if let Some(parsed_data) = &input_data.get("data") {
            let data_type = input_data["type"]
                .as_str()
                .ok_or(ParseError::ParsingError)?;

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
            let prev_seq = 0;
            let timestamp = input_data["ts"].as_i64().ok_or(ParseError::ParsingError)?;

            // Datapacket creation
            match data_type {
                "delta" => {
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
                "snapshot" => {
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
                _ => Err(ParseError::ParsingError)?,
            }
        } else {
            return Err(ParseError::ParsingError);
        }
    }
}

impl SymbolHandler for ByBitSymbolHandler {
    /// Requests all tradeable symbols from ByBit's http endpoint and parses the response into a vector of stirng. It
    /// then retains all symbols except OKBUSDT (which errors) and then creates a JSON response containing the
    /// reamining symbols as an argument.
    ///
    /// # Returns
    /// A result containing a `Value` if the response is valid and contains the necessary symbol data. If the response
    /// or parsing is unsuccessful, it will return a `SymbolError`.
    async fn get_symbols() -> Result<Symbols, SymbolError> {
        let response = match reqwest::get(BYBIT_SYMBOL_API).await {
            Ok(res) => res,
            Err(e) => return Err(SymbolError::ReqwestError(e)),
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
