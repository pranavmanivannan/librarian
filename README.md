# data-storage
Multi-exchange market data ingestion and storage service.

## Storage System
The data storage system has two core components:
- `Listener`
- `Buffer`

The core of the system revolves around the following loop:
1) Spawn in multiple exchanges which contain listeners implementing the `Listener` trait.
2) When each `Exchange` calls the `build` function, it will create two tasks, one which subscribes the listener to an
exchange and one that creates a task for pushing to a `Buffer`.
2) Each one of these listeners will be subscribed to one or more exchange endpoints to asynchronously receive messages from the exchange. (i.e. the `HuobiListener` will be subscribed to both `MarketIncremental` and `Snapshot` data).
3) These listeners will send serialize `DataPacket` structs from the messages they receive and send them over a `mpsc::channel` to a `Buffer` running in another tokio task.
4) Once a `Buffer` is full, the `Buffer` will send all data inside of it to InfluxDB and clear itself, allowing for more messages to be stored.

## Implementation Details
- On startup, the system will initially spawn in multiple `Exchange` to be used.
- Afterwards, the system will spawn in multiple listeners and consume the `UnboundedSender` spawned in by the channel. Each listener will connect to a set of endpoints and symbols.
- These exchanges return two `tokio::task::JoinHandle`, one that is for a `Buffer` to store data and one that is for the `Listener` to poll from a websocket connection.
  - In the case a connection dies within a task, the loop will create a new websocket connection.
- Alongside these listeners, there will be a `Buffer` spawned in for each listener, consuming the `UnboundedReceiver` end of the channels.
- These buffers will continuously poll from the channels and call `ingest` on the received `DataPacket`. If a `Buffer` is full, it will call `push_to_influx` and send all data currently in that buffer to InfluxDB before clearing the buffer.

### Exchange
```rust
/// Trait that owns listener and buffer and builds system for each exchange
#[async_trait]
pub trait Exchange: Sized {
    type Listener: Listener;

    /// Returns the websocket url held within the exchange. This is used when calling the listen function of the
    /// Listener for this exchange.
    fn get_socket_url(self) -> String;

    /// Creates a new Listener and Buffer using the owned channel.
    ///
    /// This will create two tasks, the first of which runs a loop which continuously polls an UnboundedReceiver
    /// for DataPackets and pushes it to a Buffer. This loop will be returned as a JoinHandle<()>. The other task
    /// creates a `Listener` will return a JoinHandle<Result<(), tungstenite::Error>>. If the creation of the task
    /// was successful, the JoinHandle can be awaited on.
    async fn build(self) -> (JoinHandle<Result<(), tungstenite::Error>>, JoinHandle<()>);
}
```

The `Listener` trait is used to abstract the logic of polling from a websocket and pushing to a channel. As the `listen` logic is the same across all exchanges, it should not need to be heavily modified. The `connect` logic should be overriden accordingly due to some exchanges requiring a different order of operations to subscribe to multiple endpoints and all symbols.

### Listener
```rust
/// The main trait of the data storage system. It holds associated types to a SymbolHandler
/// and Parser, each of which correspond to their own trait.
#[async_trait]
pub trait Listener: Send + Sync {
    type Parser: Parser;
    type SymbolHandler: SymbolHandler;
    async fn listen(
        ws_url: &str,
        sender: UnboundedSender<DataPacket>,
    )-> JoinHandle<Result<(), Error>> {
        let sender_clone = sender.clone();
        let url = ws_url.to_string();
        tokio::spawn(async move {
            loop {
                let (mut write, mut read) = Self::connect(&url).await?;
                /// asynchronously receives messages and parses them
            }
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
    fn get_symbols(
    ) -> impl std::future::Future<Output = Result<Value, SymbolError>> + std::marker::Send;
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
    /// For exchanges that need to be informed to send pings. The String will contain the pong response that we can
    /// directly send to the exchange.
    Ping(String),
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
/// Parsing errors require custom errors.
/// Creating a parsing error enum clarifies the error handling while still revealing exactly what caused the error.
#[derive(Debug)]
pub enum ParseError {
    JsonError(serde_json::Error),
    ParsingError,
}

/// When pushing to InfluxDB, there are multiple errors that may occur within one function. To elegantly handle errors,
/// the DBError is a custom error type that allows for the use of the `?` operator and allows for logging of what
/// error occurred.
pub enum DBError {
    HttpError(reqwest::StatusCode),
    ReqwestError(reqwest_middleware::Error),
    JsonError(serde_json::Error),
}

```


### Buffers
There are currently 3 exchanges, Huobi, Bybit, and Binance.
The current implementation requires 3 buffers, each with multiple vectors within it, to store both Market Incremental and Snapshot data per exchange.

```rust
/// A struct for making a buffer
pub struct Buffer {
    client: reqwest_middleware::ClientWithMiddleware,
    snapshots: Vec<String>,
    incrementals: Vec<String>,
    bucket: String,
    capacity: usize,
}

/// An implementation of the Buffer struct which allows Buffers
impl Buffer {
    /// Creates a new buffer with a reqwest client to push to InfluxDB.
    ///
    /// # Arguments
    /// * `buffer_name` - The name of the exchange.
    /// * `capacity` - The capacity of the buffer before it pushes to InfluxDB.
    pub fn new(buffer_name: &str, capacity: usize) -> Buffer {}

    /// A function that creates a new buffer and then creates a tokio::task using that buffer,
    ///
    /// # Arguments
    /// * `buffer_name` - The name of the exchange.
    /// * `capacity` - The capacity of the vectors within the buffer before they push to InfluxDB.
    /// * `receiver` - An `UnboundedReceiver` that receives the type `DataPacket`.
    ///
    /// # Returns
    /// A JoinHandle to use.
    pub fn create_task(
        buffer_name: &str,
        capacity: usize,
        receiver: UnboundedReceiver<DataPacket>,
    ) -> JoinHandle<()> {}

    /// A separate function that sorts datapackets and pushes it to buffer
    ///
    /// # Arguments
    /// * `data_packet` - A DataPacket received from a listener.
    ///
    /// # Returns
    /// A Result with an empty Ok or a DBError if the DataPacket couldn't be pushed.
    pub async fn ingest(&mut self, data_packet: DataPacket) -> Result<(), DBError> {}

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
