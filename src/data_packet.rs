use std::mem;

use serde::Serialize;
use get_size::GetSize;


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
    pub asks: Vec<(f32, f32)>,
    /// Up to top 5 bids.
    pub bids: Vec<(f32, f32)>,
    /// Current sequence number of the generated data.
    pub cur_seq: i64,
    /// Previous sequence number of the generated data. Used for keeping track of the orderbook.
    pub prev_seq: i64,
    /// Timestamp at which the exchange generated this market incremental data.
    pub timestamp: i64,
}

impl GetSize for MarketIncremental {
    fn get_heap_size(&self) -> usize {
        let symbol_pair_size = self.symbol_pair.capacity();
        let asks_size = self.asks.capacity() * mem::size_of::<(f32, f32)>();
        let bids_size = self.bids.capacity() * mem::size_of::<(f32, f32)>();
        symbol_pair_size + asks_size + bids_size
    }

    fn get_size(&self) -> usize {
        self.get_heap_size()
    }
}

/// Snapshot struct used to serialize data from orderbook snapshot endpoints on exchanges. While the `MarketIncremental`
/// and `Snapshot` structs are similar, we use the enum variants in `DataPacket` to differentiate between the two before
/// sending it to the proper buffer. This allows for reduced memory overhead in InfluxDB compared to using a flag
/// without increasing the bottleneck of the system.
#[derive(Serialize, Debug)]
pub struct Snapshot {
    /// The symbol-pair of the coin being traded.
    pub symbol_pair: String,
    /// Top 5 asks.
    pub asks: Vec<(f32, f32)>,
    /// Top 5 bids.
    pub bids: Vec<(f32, f32)>,
    /// Current sequence number of the generated data.
    pub cur_seq: i64,
    /// Previous sequence number of the generated data. Used for keeping track of the orderbook.
    pub prev_seq: i64,
    /// Timestamp at which the exchange generated this orderbook snapshot.
    pub timestamp: i64,
}

impl GetSize for Snapshot {
    fn get_heap_size(&self) -> usize {
        let symbol_pair_size = self.symbol_pair.capacity();
        let asks_size = self.asks.capacity() * mem::size_of::<(f32, f32)>();
        let bids_size = self.bids.capacity() * mem::size_of::<(f32, f32)>();
        symbol_pair_size + asks_size + bids_size
    }

    fn get_size(&self) -> usize {
        self.get_heap_size()
    }
}
