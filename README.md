# data-storage
Multi-exchange market data ingestion and storage service.

## Internal Engine
The data storage system has two core components:
- Listeners
- Buffers

The core of the system revolves around spawning in listeners which will be put into tokio tasks
to continuously run. Each listener will be subscribed to one or more exchange endpoints. For example,
the Huobi Market Listener can be subscribed to both the Market Incremental and Market Snapshot endpoints.
These listeners will send DataPackets over a channel to a buffer running in another tokio task, where these buffers
will possibly sort the messages. Once a buffer is full, the buffer will send all data inside of it to InfluxDB and
clear itself, allowing for more messages to be stored.

### Main
```rust
/// Sets up a logger and corresponding log file, then spawns in listeners and buffers.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {}
```

### Listener
```rust
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
    fn split(self) -> (R, W);
    async fn listen(self, sender: UnboundedSender<DataPacket>) -> JoinHandle<Result<(), Error>> {
        let (mut r, _) = self.split();
        tokio::spawn(async move {
            /// loops while we get messages
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
        /// to implement
    }
}

/// The Parser trait is custom implemented for each exchange and endpoint.
/// It returns a Result, either containing a valid DataPacket to be sent to the buffer
/// or a ParseError due to invalid data in the message.
pub trait Parser {
    fn parse(message: Message) -> Result<DataPacket, ParseError>;
}

/// The SymbolHandler trait is custom implemented for each exchange and endpoint.
/// It returns a Result, containing either a valid Value that can be sent to subscribe
/// to all symbols or a SymbolError.
pub trait SymbolHandler {
    async fn get_symbols() -> Result<Value, SymbolError>;
}

```



### Data Packet
```rust
/// Data Packet is an enum that differentiates between the type of data, and the data inside contains relavent data sent to influx that can be used by data query team.
pub enum DataPacket {
    MI(MarketIncremental),
    ST(Snapshot),
    Ping(i64), // for houbi, to flag a response with pongs
    Invalid,   // for invalid data, such as missing sequence number
}


pub struct Snapshot {
    pub symbol_pair: String,
    pub asks: Vec<Value>,
    pub bids: Vec<Value>,
    pub cur_seq: i64,
    pub prev_seq: i64,
    pub timestamp: i64,
}
```

### Error Handling
```rust
/// When getting symbols there are two types of errors: reqwest errors and reading errors.
/// Reading errors require custom errors.
/// Creating a symbol error enum clarifies the error handling while still revealing exactly what caused the error.
#[derive(Debug)]
pub enum SymbolError {
    ReqwestError(reqwest::Error),
    MissingSymbolsError,
}

/// When parsing there are two types of errors: json errors and parsing errors.
/// parsing errors require custom errors.
/// Creating a parsing error enum clarifies the error handling while still revealing exactly what caused the error.
#[derive(Debug)]
pub enum ParseError {
    JsonError(serde_json::Error),
    ParsingError,
}

/// Same for DB errors
pub enum DBError {
    HttpError(reqwest::StatusCode),
    ReqwestError(reqwest_middleware::Error),
    JsonError(serde_json::Error),
}

```



### Buffers
6 buffers: Huobi, Bybit, and Binance; Market incrementals and snapshots for each
```rust
/// A struct for making a buffer
pub struct Buffer {
    client: reqwest_middleware::ClientWithMiddleware,
    storage: Vec<String>,
    bucket: String,
    capacity: usize,
}

/// An implementation of the Buffer struct which allows Buffers
impl Buffer {
    /// Creates a new buffer with a reqwest client to push to InfluxDB.
    ///
    /// # Arguments
    /// * `bucket_name` - The name of the bucket on InfluxDB.
    /// * `capacity` - The capacity of the buffer before it pushes to InfluxDB.
    pub fn new(bucket_name: &str, capacity: usize) -> Buffer {}

    /// A separate function that sorts datapackets and pushes it to buffer
    ///
    /// # Arguments
    /// * `data_packet` - A DataPacket received from a listener.
    ///
    /// # Returns
    /// A Result with an empty Ok or a DBError if the DataPacket couldn't be pushed.
    pub async fn ingest(&mut self, data_packet: DataPacket) -> Result<(), DBError> {}

    /// Pushes data from a buffer to an InfluxDB bucket and clears the buffer afterwards.
    /// Pushes data from a buffer to an InfluxDB bucket and clears the buffer afterwards.
    ///
    /// # Arguments
    /// * `message` - A string of the datapacket formatted to fit InfluxDB.
    ///
    /// # Returns
    /// A Result that is either empty or a DBError if the message couldn't be pushed to a buffer.
    pub async fn push_and_flush(&mut self, message: String) -> Result<(), DBError> {}

    /// Pushes the data in a buffer to an InfluxDB bucket.
    async fn push_to_influx(&self) -> Result<(), DBError> {}
}
```