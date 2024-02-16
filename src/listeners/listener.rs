use crate::{
    data_packet::DataPacket,
    error::{ParseError, SymbolError},
};
use async_trait::async_trait;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde_json::Value;
use tokio::{net::TcpStream, sync::mpsc::UnboundedSender, task::JoinHandle};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::{Error, Message};

/// The main trait of the data storage system. It holds associated types to a SymbolHandler
/// and Parser, each of which correspond to their own trait.
#[async_trait]
pub trait Listener: Send + Sync {
    type Parser: Parser;
    type SymbolHandler: SymbolHandler;
    /// Creates a the read and write halves of the websocket stream to receive messages and then
    /// parses those messages. If the message is valid, it will be sent across the channel.
    ///
    /// # Arguments
    /// * `sender` - An UnboundedSender of the type DataPacket. The corresponding UnboundedReceiveris in a storage_loop.
    ///
    /// # Returns
    /// A JoinHandle to be awaited on within a tokio::task.
    async fn listen(
        sender: UnboundedSender<DataPacket>,
    ) -> JoinHandle<Result<(), tungstenite::Error>> {
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            loop {
                let (mut write, mut read) = match Self::connect().await {
                    Ok(stream_tuple) => stream_tuple,
                    Err(_e) => {
                        log::error!("{} - Reconnecting...", Self::exchange_name());
                        continue;
                    }
                };
                log::info!(
                    "{} - Websocket connection established!",
                    Self::exchange_name()
                );
                while let Some(Ok(message)) = read.next().await {
                    if let Message::Close(_) = message {
                        log::info!("{} - Websocket connection closed!", Self::exchange_name());
                        break;
                    }
                    let data_packet = Self::Parser::parse(message);
                    if let Ok(data_packet) = data_packet {
                        match data_packet {
                            DataPacket::Ping(pong) => {
                                let _ = write.send(Message::Text(pong)).await;
                            }
                            _ => {
                                let _ = sender_clone.send(data_packet);
                            }
                        }
                    }
                }
            }
        })
    }

    /// Uses a url to connect to a valid WebSocketStream and then splits it into the read and write halves
    /// of the stream. Each listener should have a custom implementation that overrides this function.
    ///
    /// # Arguments
    /// * `websocket_url` - A &str containing a valid websocket url to connect to.
    ///
    /// # Returns
    /// Two halves of a WebSocketStream.
    async fn connect() -> Result<
        (
            SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ),
        Error,
    >;

    /// This function is utilized when logging which exchange is reconnecting during the `listen` function.
    ///
    /// # Returns
    /// A string containing the name of the exchange.
    fn exchange_name() -> String;
}

/// The `Parser` trait contains the singular parse function that is custom implemented
/// for each exchange and endpoint.
pub trait Parser {
    /// Parses a tungstenite Message.
    ///
    /// # Arguments
    /// * `message` - A message of the `Message` type
    ///
    /// # Returns
    /// A Result containing a `DataPacket` if parsing was successful, else a `ParseError`.
    fn parse(message: Message) -> Result<DataPacket, ParseError>;
}

/// The `SymbolHandler` trait is custom implemented for each exchange and endpoint.
pub trait SymbolHandler {
    /// Requests all tradeable symbols from an exchange's http endpoint and parses the response.
    ///
    /// # Returns
    /// A result containing a `Value` if the response is valid and contains the necessary symbol data. Else, it will
    /// return a `SymbolError`. The `Value` will contain the necessary string used to subscribe to all symbols.
    fn get_symbols(
    ) -> impl std::future::Future<Output = Result<Symbols, SymbolError>> + std::marker::Send;
}

/// The `Symbols` enum is used when returning the result of the `get_symbols` function. This allows for extensibility
/// when adding additional exchanges, as they may require different symbol formats to be sent when subscribing to all
/// symbols.
pub enum Symbols {
    SymbolVector(Vec<String>),
    SymbolString(String),
}

/// Parses the first 5 bids and asks from an array inside the JSON message the exchange sends. This array is
/// converted to a Vector of `Value` types before being input into this function.
///
/// # Arguments
/// * `data_vector` - A reference to a `Vec<Value>` which is a variable-length vector. Market incremental data
/// may contain less than 5 bids or asks, but snapshot data will contain at least 5 per both bids and asks.
///
/// # Returns
/// A `Vec<Value>` containing up to the first 5 bids or asks.
pub fn parse_bids_asks(data_vector: &Vec<Value>) -> Vec<(f32, f32)> {
    let mut data_count = 0;
    let mut res_vector: Vec<(f32,f32)> = Vec::new();
    for data in data_vector.iter() {
        if data_count >= 5 {
            break;
        }
        if let (Some(price), Some(quantity)) = (data[0].as_f64(), data[1].as_f64()) {
            let data_pair: (f32, f32) = (price as f32, quantity as f32);
            res_vector.push(data_pair);
        } else if let (Some(price), Some(quantity)) = (
            data[0].as_str().and_then(|s| s.parse::<f32>().ok()),
            data[1].as_str().and_then(|s| s.parse::<f32>().ok())
        ) {
            let data_pair: (f32, f32) = (price, quantity);
            res_vector.push(data_pair);
        }
        data_count += 1;
    }
    res_vector
}

