use super::{
    bybit_listener::{bybit_parser, ByBitListener, ByBitSymbolHandler},
    listener::{Listener, Parser},
};
use crate::{data_packet::DataPacket, error::ParseError};
use async_trait::async_trait;
use futures_util::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::{Error, Message};

pub struct ByBitSnapshotListener {}
pub struct ByBitSnapshotParser {}

#[async_trait]
impl Listener for ByBitSnapshotListener {
    type Parser = ByBitSnapshotParser;
    type SymbolHandler = ByBitSymbolHandler;

    async fn connect() -> Result<
        (
            SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ),
        Error,
    > {
        ByBitListener::connect().await
    }

    fn exchange_name() -> String {
        ByBitListener::exchange_name()
    }
}

impl Parser for ByBitSnapshotParser {
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
            if data_type == "snapshot" {
                bybit_parser(&input_data, parsed_data, "snapshot")
            } else {
                return Err(ParseError::ParsingError);
            }
        } else {
            return Err(ParseError::ParsingError);
        }
    }
}
