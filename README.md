# data-storage
Multi-exchange market data ingestion and storage service.

## Storage System
The data storage system has two core components:
- `Listener`
- `Buffer`

The core of the system revolves around the following loop:
1) Spawn in multiple listeners which implement the `Listener` trait.
2) Subscribe each listener to one or more exchange endpoints to asynchronously receive messages from the exchange. (i.e. the `HuobiListener` will be subscribed to both `MarketIncremental` and `Snapshot` data).
3) Each listener will send serialize `DataPacket` structs from the messages they receive and send them over a `mpsc::channel` to a `Buffer` running in another tokio task.
4) Once a `Buffer` is full, the `Buffer` will send all data inside of it to InfluxDB and clear itself, allowing for more messages to be stored.

## Implementation Details
- On startup, the system will initially spawn in multiple `mpsc::channel` to be used.
- Afterwards, the system will spawn in multiple listeners and consume the `UnboundedSender` spawned in by the channel. Each listener will connect to a set of endpoints and symbols.
- These listeners return a `tokio::task::JoinHandle`, and each listener will be awaited on within a `tokio::task`.
  - In the case a connection dies within a task, the loop will create a new task and respawn the listener task before awaiting it.
- Alongside these listeners, there will be a `Buffer` spawned in for each listener, consuming the `UnboundedReceiver` end of the channels.
- These buffers will continuously poll from the channels and call `ingest` on the received `DataPacket`. If a `Buffer` is full, it will call `push_to_influx` and send all data currently in that buffer to InfluxDB before clearing the buffer.

### Exchange
```rust
/// Trait that owns listener and buffer and builds system for each exchange
#[async_trait]
pub trait Exchange {
    type Listener: Listener;
    type BufferHandler: BufferHandler;

    /// calls Listener::listen and BufferHandler::new
    async fn build() -> Result<JoinHandle, Error>;
}

/// The Listener trait contains listen which connects to a websocket and listens and parses data
pub trait Listener {
    fn listen(message: Message) -> Result<DataPacket, ParseError>;
}

/// The BufferHandler trait contains buffer which instantiates buffer and sends data to Influx.
pub trait BufferHandler {
    async fn build_buffer() -> Buffer;
}

```

### Listener
```rust
/// The main trait of the data storage system. It holds associated types to a SymbolHandler
/// and Parser, each of which correspond to their own trait.
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
            /// asynchronously receives messages and parses them
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
    >
}
```

The following two traits are traits used within the `Listener` trait. The functions for both traits are meant to be
implemented on an exchange and endpoint basis.

```rust

/// The Parser trait contains the singular parse function which is custom implemented
/// for each exchange and endpoint.
pub trait Parser {
    fn parse(message: Message) -> Result<DataPacket, ParseError>;
}

/// The SymbolHandler trait contains the get_symbols method which is custom implemented
/// for each exchange and endpoint.
pub trait SymbolHandler {
    async fn get_symbols() -> Result<Value, SymbolError>;
}

```


### Data Packet
The `DataPacket` Enum is crucial as it is how data is serialized once a `Message` has been read by a listener. Below is an example of the `Snapshot` struct of the `ST` variant in the enum.

```rust
/// The DataPacket Enum contains various structs. This allows for the `Parser` trait to parse a `Message` from any
/// endpoint and return a singular data type that can be sent over a `channel`.
#[derive(Debug)]
pub enum DataPacket {
    /// Serializes market incremental data.
    MI(MarketIncremental),
    /// Serializes orderbook snapshots.
    ST(Snapshot),
    /// For exchanges that need to be informed to send pings. The i64 will contain the pong response.
    Ping(i64),
    /// Flags data as invalid.
    Invalid,
}

/// Snapshot struct used to serialize data from orderbook snapshot endpoints on exchanges.
#[derive(Serialize, Debug)]
pub struct Snapshot {
    /// The symbol-pair of the coin being traded.
    pub symbol_pair: String,
    /// Top 5 asks.
    pub asks: Vec<Value>,
    /// Top 5 bids.
    pub bids: Vec<Value>,
    /// Current sequence number of the generated data.
    pub cur_seq: i64,
    /// Previous sequence number of the generated data. Used for keeping track of the orderbook.
    pub prev_seq: i64,
    /// Timestamp at which the exchange generated this orderbook snapshot.
    pub timestamp: i64,
}
```

### Error Handling
There are multiple errors that can occur during retrieving symbols, parsing messages, or pushing to the buffers/InfluxDB. In order to account for this during runtime without using heap allocations caused by `Box<dyn std::error::Error>`, custom error types have been provided.

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
There are currently 3 exchanges, Huobi, Bybit, and Binance.
The current implementation will require 6 buffers to store both Market Incremental and Snapshot data for each.

- TODO: Possibly make each buffer contain multiple vectors instead and sort the data within the buffer per exchange.

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
