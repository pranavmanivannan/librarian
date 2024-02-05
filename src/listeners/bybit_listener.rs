use crate::data_packet::DataPacket;
use crate::data_packet::MarketIncremental;
use crate::error::ParseError;
use crate::error::SymbolError;
use async_trait::async_trait;
use futures::{Sink, Stream};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt,
};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;

use super::listener::{Listener, Parser, SymbolHandler};

/// The websocket url used to connect to ByBit's perpetuals and futures market data.
const BYBIT_WS: &str = "wss://stream.bybit.com/v5/public/linear";
/// The http url used to request all symbols on ByBit's market.
const BYBIT_SYMBOL_API: &str = "https://api-testnet.bybit.com/v5/market/tickers?category=linear";

pub struct ByBitListener<
    R: Stream<Item = Result<Message, tungstenite::Error>> + Send + 'static,
    W: Sink<Message> + Unpin + Send + 'static,
> {
    read: R,
    write: W,
}

pub struct ByBitParser {}
pub struct ByBitSymbolHandler {}

impl
    ByBitListener<
        SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    >
{
    pub async fn setup() -> Result<Self, tungstenite::Error> {
        let (mut write, read) = Self::connect(BYBIT_WS).await?;
        let symbols = ByBitSymbolHandler::get_symbols().await;
        if let Ok(symbols) = symbols {
            let _ = write.send(Message::Text(symbols.to_string())).await;
        }
        Ok(Self { read, write })
    }
}

#[async_trait]
impl<
        R: Stream<Item = Result<Message, tungstenite::Error>> + Unpin + Send + 'static,
        W: Sink<Message> + Unpin + Send + 'static,
    > Listener<R, W> for ByBitListener<R, W>
{
    type Parser = ByBitParser;
    type SymbolHander = ByBitSymbolHandler;

    fn split(self) -> (R, W) {
        (self.read, self.write)
    }
}

impl Parser for ByBitParser {
    fn parse(message: Message) -> Result<DataPacket, ParseError> {
        let message_string = message.to_string();
        let input_data: serde_json::Value =
            serde_json::from_str(&message_string).map_err(ParseError::JsonError)?;

        if !input_data.is_null() && input_data["op"] != "subscribe" {
            let parsed_data = &input_data["data"];

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

            let enum_creator = MarketIncremental {
                symbol_pair: symb_pair,
                asks,
                bids,
                cur_seq: seq_num,
                prev_seq: 0,
                timestamp: ts,
            };

            Ok(DataPacket::MI(enum_creator))
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
