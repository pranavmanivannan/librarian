use std::fmt;

/// Custom error type used to handle errors that may occur when retrieving symbols from an exchange's HTTP endpoint.
#[derive(Debug)]
pub enum SymbolError {
    /// Wraps reqwest errors that occur when either the request to retrieve all symbols fails or deserializing the
    /// response containing symbols fails.
    ReqwestError(reqwest::Error),
    /// Wraps errors that occur when the symbols field is missing from the response or when the corresponding field(s)
    /// are not contained within the response.
    MissingSymbolsError,
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SymbolError::ReqwestError(ref error) => write!(f, "Reqwest Error: {}", error),
            SymbolError::MissingSymbolsError => write!(f, "Symbol Parsing Error"),
        }
    }
}

/// Custom error type used to handle errors that may occur when parsing messages from a websocket connection.
#[derive(Debug)]
pub enum ParseError {
    /// Wraps errors that occur when trying to parse JSON data from a websocket message converted to a string.
    JsonError(serde_json::Error),
    /// Wraps errors that occur when trying to parse messages but are not of a specific type. This variant is usually
    /// used when deserialization of a field fails, usually when converting from a `Value` to an `Option` and the
    /// `Option` contains a `None` instead of an expected value.
    ParsingError,
    /// Wraps errors that occur when trying to convert a `Vec<u8>` to a `String`. This error usually occurs when a gzip
    /// compressed message is decompressed and the resulting `Vec<u8>` is not a valid UTF-8 string.
    Utf8Error(std::string::FromUtf8Error),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ParseError::JsonError(ref error) => write!(f, "Json Error: {}", error),
            ParseError::ParsingError => write!(f, "Parsing Error"),
            ParseError::Utf8Error(ref error) => write!(f, "UTF-8 Error: {}", error),
        }
    }
}

/// Custom error type used to handle errors that may occur when pushing to a buffer or InfluxDB.
#[derive(Debug)]
pub enum DBError {
    /// Wraps reqwest HTTP errors when pushing to InfluxDB. Usually occurs due to a 404 meaning the bucket does not
    /// exist or the exchange name used as input in `build` for the `Exchange` trait is incorrect.
    HttpError(reqwest::StatusCode),
    /// Wraps reqwest errors that occur after pushing to InfluxDB. This usually occurs when trying to read the HTTP
    /// response text.
    ReqwestError(reqwest::Error),
    /// Wraps the reqwest middleware client errors that occur when the response itself does not go through or is not
    /// successful.
    ReqwestMiddlewareError(reqwest_middleware::Error),
    /// Wraps JSON errors that occur when trying to parse and format parts of the DataPacket.
    JsonError(serde_json::Error),
}

impl fmt::Display for DBError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DBError::HttpError(ref status) => write!(f, "HTTP Error: {}", status),
            DBError::ReqwestError(ref error) => write!(f, "Reqwest Error: {}", error),
            DBError::ReqwestMiddlewareError(ref error) => {
                write!(f, "Reqwest middleware Error: {}", error)
            }
            DBError::JsonError(ref error) => write!(f, "JSON Error: {}", error),
        }
    }
}
