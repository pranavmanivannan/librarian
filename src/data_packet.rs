use serde::Serialize;
use serde_json::Value;

/// The DataPacket Enum contains various structs. This allows for the `Parser` trait to parse a `Message` from any
/// endpoint and return a singular data type that can be sent over a `channel`.
#[derive(Debug)]
pub enum DataPacket {
    /// Serializes market incremental data.
    MI(MarketIncremental),
    /// Serializes orderbook snapshots.
    ST(Snapshot),
    /// For exchanges that need to be informed to send pings. The i64 will contain the pong response.
    Ping(String),
}

/// Market Incremental struct used to serialize data from market incremental endpoints on exchanges.
#[derive(Serialize, Debug)]
pub struct MarketIncremental {
    /// The symbol-pair of the coin being traded.
    pub symbol_pair: String,
    /// Up to top 5 asks.
    pub asks: Vec<Value>,
    /// Up to top 5 bids.
    pub bids: Vec<Value>,
    /// Current sequence number of the generated data.
    pub cur_seq: i64,
    /// Previous sequence number of the generated data. Used for keeping track of the orderbook.
    pub prev_seq: i64,
    /// Timestamp at which the exchange generated this market incremental data.
    pub timestamp: i64,
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

// snapshot and mi are copy pasted structs, but this is better than using a flag
// in case we add stuff like BBO and TD