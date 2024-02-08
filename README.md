# data-storage
Multi-exchange market data ingestion and storage service.

## Storage System
Core components:
- `Exchange`
- `Listener`
- `Buffer`

## How it works:
1) Call the `build` function for each exchange implementing the `Exchange` trait. This will create two tasks of the type `tokio::task::JoinHandle`.
2) One of these tasks will subscribe the `Listener` for the exchange to all necessary endpoints and symbols. This task will also contain an `UnboundedReceiver` to send the parsed messages over a `mpsc::channel`.
3) The other task will repeatedly poll an `UnboundedReceiver` for `DataPackets` and will push this data to the correct `Buffer` depending on the type of data (e.g., snapshot or incremental).
4) Once a `Buffer` is full, that `Buffer` will send all data inside to InfluxDB and clear itself, allowing for more data packets to be stored.

## Implementation Details
- Each exchange (Huobi, Binance, ByBit, etc.) should only have one corresponding exchange file implementing `Exchange`. 
- Each exchange should only have one listener with a websocket connection. All logic for multiple websocket endpoints and symbols should be handled when calling `connect`. Http polling is handled using a separate connection.
- Additionally, each exchange has multiple buffers. The current implementation requires 6 total buffers as we store 2 endpoints (market incremental and snapshot) per exchange. No more than one background task holding these buffers should exist per exchange.
- In the case a websocket connection dies, the loop within the `tokio::task` will create a new websocket connection. This occurs by recreating the `WebSocketStream` connection and splitting it, rather than recreating the entire listener.

### Exchange
The `Exchange` trait is used to build a comprehensive connection and storage loop per exchange. For exchanges such as Binance which may require multiple listeners (websocket for market incremental and http for snapshot data), a custom implementation of the exchange can be used instead.
```rust
/// Trait that owns listener and buffer and builds system for each exchange
#[async_trait]
pub trait Exchange: Sized {
    type Listener: Listener;

    /// Creates a new Listener and Buffer using the owned channel.
    ///
    /// This will create two tasks, the first of which runs a loop which continuously polls an UnboundedReceiver
    /// for DataPackets and pushes it to a Buffer. This loop will be returned as a JoinHandle<()>. The other task
    /// creates a `Listener` will return a JoinHandle<Result<(), tungstenite::Error>>. If the creation of the task
    /// was successful, the JoinHandle can be awaited on. The buffer_name is used to clarify which exchange this buffer
    /// belongs to and helps with logic in pushing to InfluxDB.
    async fn build(buffer_name: &str) -> (JoinHandle<Result<(), tungstenite::Error>>, JoinHandle<()>);
}
```

### Listener
The `Listener` trait is used to abstract the logic of polling from a websocket and pushing to a channel. As the `listen` logic is the same across all exchanges, it should not need to be heavily modified. The `connect` logic should be overriden accordingly per exchange due to endpoint formats and symbol subscriptions varying across exchanges.

```rust
/// The main trait of the data storage system. It holds associated types to a SymbolHandler
/// and Parser, each of which correspond to their own trait.
#[async_trait]
pub trait Listener: Send + Sync {
    type Parser: Parser;
    type SymbolHandler: SymbolHandler;
    async fn listen(
        sender: UnboundedSender<DataPacket>,
    )-> JoinHandle<Result<(), Error>> {
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            loop {
                let (mut write, mut read) = Self::connect().await?;
                /// asynchronously receives messages and parses them
            }
        })
    }

    /// This function will be custom implemented per exchange. There is no websocket url passed in as an argument
    /// as the listener.rs file for the exchange will contain it as a constant.
    async fn connect() ->
        Result<
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
    ) -> impl std::future::Future<Output = Result<Symbols, SymbolError>> + std::marker::Send;
}

```

As shown in `SymbolHandler`, we use a `Symbols` enum to allow for multiple types of symbol formats to be passed in. This is due to the varying format of subscriptions across exchanges. For example, subscribing to all symbols for ByBit requires us to only send a single message, whereas Huobi requires us to manually send a subscription for each symbol.

```rust
pub enum Symbols {
  SymbolVector(Vec<String>),
  SymbolString(String),
}
```


### Data Packet
The `DataPacket` Enum is crucial as it is how data is serialized once a `Message` has been read by a listener. MarketIncremental and Snapshot are identical structs, but this allows for us to sort them based on the type of enum within the `Buffer` and sending the data to two separate buckets on InfluxDB. This reduces the memory overhead on the database for storing a flag to inform us of what type of data it is. Below is an example of the `Snapshot` struct of the `ST` variant in the enum.

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
There are multiple errors that can occur during retrieving symbols, parsing messages, or pushing to the buffers/InfluxDB. In order to account for this during runtime without using heap allocations caused by `Box<dyn std::error::Error>`, custom error types have been created. 

```rust
/// When getting symbols there are two types of errors: reqwest errors and reading errors.
/// Reading errors require custom errors.
/// Creating a symbol error enum clarifies the error handling while still revealing exactly what caused the error.
#[derive(Debug)]
pub enum SymbolError {
    ReqwestError(reqwest::Error),
    MissingSymbolsError,
}

/// When parsing there are multiple types of errors including json, utf8, and parsing errors.
/// Creating a parsing error enum clarifies the error handling while still revealing exactly what caused the error.
#[derive(Debug)]
pub enum ParseError {
    JsonError(serde_json::Error),
    ParsingError,
    Utf8Error(std::string::FromUtf8Error),
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
There are currently 3 exchanges, Huobi, Bybit, and Binance. The current implementation requires 6 buffers to store both Market Incremental and Snapshot data per exchange. In the case of additional endpoints, adding another buffer to the exchange's corresponding background task will easily scale it.

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
    /// * `capacity` - The maximum capacity of the buffer.
    pub fn new(buffer_name: &str, capacity: usize) -> Buffer {}

    /// A function that creates a new buffer and then creates a tokio::task using that buffer,
    ///
    /// # Arguments
    /// * `buffer_name` - The name of the exchange.
    /// * `capacity` - The maximum capacity of the buffer (size of backing vector).
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
    /// A Result with an empty Ok or a DBError if the DataPacket or Buffer couldn't be pushed.
    pub async fn ingest(&mut self, data_packet: DataPacket) -> Result<(), DBError> {}

    /// Pushes the data in a buffer to an InfluxDB bucket.
    async fn push_to_influx(&self) -> Result<(), DBError> {}
}
```
