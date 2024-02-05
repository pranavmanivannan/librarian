use serde::Serialize;
use serde_json::Value;

#[derive(Debug)]
pub enum DataPacket {
    MI(MarketIncremental),
    ST(Snapshot),
    Ping(i64), // for houbi, to flag a response with pongs
    Invalid,   // just in case
}

#[derive(Serialize, Debug)]
pub struct MarketIncremental {
    pub symbol_pair: String,
    pub asks: Vec<Value>,
    pub bids: Vec<Value>,
    pub cur_seq: i64,
    pub prev_seq: i64,
    pub timestamp: i64,
}

#[derive(Serialize, Debug)]
pub struct Snapshot {
    pub symbol_pair: String,
    pub asks: Vec<Value>,
    pub bids: Vec<Value>,
    pub cur_seq: i64,
    pub prev_seq: i64,
    pub timestamp: i64,
}

// snapshot and mi are copy pasted structs, but this is better than using a flag
// in case we add stuff like BBO and TD
