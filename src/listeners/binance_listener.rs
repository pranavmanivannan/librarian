use crate::data_packet::DataPacket;
use crate::data_packet::MarketIncremental;
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

const BINANCE_SYMBOL_API: &str = "https://api.binance.us/api/v3/exchangeInfo";

pub struct BinanceListener {}
pub struct BinanceParser {}
pub struct BinanceSymbolHandler {}

#[async_trait]
impl Listener for BinanceListener {
    type Parser = BinanceParser;
    type SymbolHandler = BinanceSymbolHandler;

    async fn connect(
        websocket_url: &str,
    ) -> Result<
        (
            SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ),
        Error,
    > {
        let symbols = Self::SymbolHandler::get_symbols().await;
        if let Ok(Symbols::SymbolString(symbols)) = symbols {
            let binance_url = format!("{}/stream?streams={}", websocket_url, symbols.to_string());
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

        let symbols = symbol_list.join("/");

        Ok(Symbols::SymbolString(symbols))
    }
}
