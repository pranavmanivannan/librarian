use crate::data_packet::DataPacket;
use crate::error::ParseError;
use crate::error::SymbolError;
use async_trait::async_trait;
use futures::{Sink, Stream};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::error::Error as TungsteniteError;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::{Error, Message};
use url::Url;

/// The main trait of the data storage system. It holds associated types to a SymbolHandler
/// and Parser, each of which correspond to their own trait.
///
/// Using split() will consumes the read and write streams spawned in by the listener.
///
/// Calling listen() will spawn a tokio task that can be awaited on. The task will asynchronously poll
/// from the websocket readstream and send the data to the corresponding exchange's buffer.
///
/// Calling connect() returns a split socket stream, the write and read streams respectively.
#[async_trait]
pub trait Listener<
    R: Stream<Item = Result<Message, Error>> + Unpin + Send + 'static,
    W: Sink<Message> + Unpin + Send + 'static,
>: Sized
{
    type Parser: Parser;
    type SymbolHander: SymbolHandler;
    /// Consumes the listener's read and writestreams for use.
    fn split(self) -> (R, W);
    async fn listen(self, sender: UnboundedSender<DataPacket>) -> JoinHandle<Result<(), Error>> {
        let (mut r, _) = self.split();
        tokio::spawn(async move {
            while let Some(Ok(message)) = r.next().await {
                if let Message::Close(_) = message {
                    break;
                } else {
                    let data_packet = Self::Parser::parse(message);
                    if let Ok(data_packet) = data_packet {
                        let _ = sender.send(data_packet);
                    }
                }
            }
            Ok::<(), Error>(())
        })
    }

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

/// The Parser trait is custom implemented for each exchange and endpoint.
/// It returns a Result, either containing a valid `DataPacket` to be sent to the buffer
/// or a `ParseError` due to invalid data in the message.
pub trait Parser {
    /// Parses a tungstenite Message and returns a `DataPacket` if successful or a `ParseError`
    /// if an error was occurred.
    fn parse(message: Message) -> Result<DataPacket, ParseError>;
}

/// The `SymbolHandler` trait is custom implemented for each exchange and endpoint.
/// It returns a Result, containing either a valid Value that can be sent to subscribe
/// to all symbols or a `SymbolError`.
pub trait SymbolHandler {
    async fn get_symbols() -> Result<Value, SymbolError>;
}
