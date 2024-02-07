use crate::data_packet::DataPacket;
use crate::error::ParseError;
use crate::error::SymbolError;
use async_trait::async_trait;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::error::Error as TungsteniteError;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::{Error, Message};
use url::Url;

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
        ws_url: &str,
        sender: UnboundedSender<DataPacket>,
    ) -> JoinHandle<Result<(), tungstenite::Error>> {
        let sender_clone = sender.clone();
        let url = ws_url.to_string();
        tokio::spawn(async move {
            loop {
                let (mut write, mut read) = Self::connect(&url).await?;
                while let Some(Ok(message)) = read.next().await {
                    if let Message::Close(_) = message {
                        break;
                    } else {
                        let data_packet = Self::Parser::parse(message);
                        if let Ok(data_packet) = data_packet {
                            match data_packet {
                                DataPacket::Ping(pong) => {
                                    let _ = write.send(Message::Text(pong)).await;
                                    println!("Pong sent");
                                }
                                _ => {
                                    let _ = sender_clone.send(data_packet);
                                }
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
        let (write, read) = socket.split();
        return Ok((write, read));
    }
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

pub enum Symbols {
    SymbolVector(Vec<String>),
    SymbolString(String),
}
