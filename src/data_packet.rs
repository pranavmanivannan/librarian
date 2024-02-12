use serde::Serialize;
use serde_json::Value;
use crate::{buffer::DataType, error::ParseError};

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


// Custom deserializer
// fields: symbol pair, asks, bids, seq, prev_seq, timestamp
// use square bracket notation here because caller verifies structure of json_data
pub fn deserialize_packet(json_data: &Value, fields: &[&str], data_type: DataType) -> Result<DataPacket, ParseError> {

    // field[0]: symbol pair
    let symbol_pair = match fields[0] {
        "NULL" => "NULL".to_string(),
        _ => json_data[fields[0]]
            .as_str()
            .ok_or(ParseError::ParsingError)?
            .to_uppercase(),
    };

    // field[1]: asks
    let ask_vector = json_data[fields[1]]
        .as_array()
        .ok_or(ParseError::ParsingError)?;
    let asks: Vec<Value> = if ask_vector.len() >= 5 {
        ask_vector[..5].to_vec()
    } else {
        ask_vector.to_vec()
    };

    // field[2]: bids
    let bid_vector = json_data[fields[2]]
        .as_array()
        .ok_or(ParseError::ParsingError)?;
    let bids: Vec<Value> = if bid_vector.len() >= 5 {
        bid_vector[..5].to_vec()
    } else {
        bid_vector.to_vec()
    };

    // field[3]: curseq
    let cur_seq = json_data[fields[3]]
        .as_i64()
        .ok_or(ParseError::ParsingError)?;

    // field[4]: prevseq
    let prev_seq = match fields[4] {
        "NULL" => 0,
        _ => json_data[fields[4]]
            .as_i64()
            .ok_or(ParseError::ParsingError)?,
    };

    // field[5]: timestamp
    let timestamp = match fields[5] {
        "NULL" => 0,
        _ => json_data[fields[5]]
            .as_i64()
            .ok_or(ParseError::ParsingError)?,
    };

    // Datapacket creation
    match data_type {
        DataType::MI => {
            let enum_creator = MarketIncremental {
                symbol_pair,
                asks,
                bids,
                cur_seq,
                prev_seq,
                timestamp,
            };

            return Ok(DataPacket::MI(enum_creator))
        },
        DataType::ST =>{
                let enum_creator = Snapshot {
                symbol_pair,
                asks,
                bids,
                cur_seq,
                prev_seq,
                timestamp,
            };

            return Ok(DataPacket::ST(enum_creator))
        },
    }
}