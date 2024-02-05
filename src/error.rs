use std::fmt;

#[derive(Debug)]
pub enum SymbolError {
    ReqwestError(reqwest::Error),
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

#[derive(Debug)]
pub enum ParseError {
    JsonError(serde_json::Error),
    ParsingError,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ParseError::JsonError(ref error) => write!(f, "Json Error: {}", error),
            ParseError::ParsingError => write!(f, "Parsing Error"),
        }
    }
}

#[derive(Debug)]
pub enum DBError {
    HttpError(reqwest::StatusCode),
    ReqwestError(reqwest::Error),
    ReqwestMiddlewareError(reqwest_middleware::Error),
    JsonError(serde_json::Error),
}

impl fmt::Display for DBError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DBError::HttpError(ref status) => write!(f, "HTTP Error: {}", status),
            DBError::ReqwestError(ref error) => write!(f, "Reqwest Error: {}", error),
            DBError::ReqwestMiddlewareError(ref error) => write!(f, "Reqwest middleware Error: {}", error),
            DBError::JsonError(ref error) => write!(f, "JSON Error: {}", error),
        }
    }
}
