use reqwest::StatusCode;
use std::{fmt, num::ParseFloatError};

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
    ReqwestError(reqwest_middleware::Error),
    JsonError(serde_json::Error),
}

impl fmt::Display for DBError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DBError::HttpError(ref status) => write!(f, "HTTP Error: {}", status),
            DBError::ReqwestError(ref error) => write!(f, "Reqwest Error: {}", error),
            DBError::JsonError(ref error) => write!(f, "JSON Error: {}", error),
        }
    }
}

impl std::error::Error for DBError {}

impl From<reqwest::Error> for DBError {
    fn from(error: reqwest::Error) -> Self {
        DBError::ReqwestError(reqwest_middleware::Error::Reqwest(error))
    }
}

impl From<StatusCode> for DBError {
    fn from(status: StatusCode) -> Self {
        DBError::HttpError(status)
    }
}

impl From<serde_json::Error> for DBError {
    fn from(error: serde_json::Error) -> Self {
        DBError::JsonError(error)
    }
}

#[derive(Debug)]
pub struct ParseErr {
    pub message: &'static str,
}

impl std::fmt::Display for ParseErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse Error: {}", self.message)
    }
}

impl std::error::Error for ParseErr {}

impl From<serde_json::Error> for ParseErr {
    fn from(_error: serde_json::Error) -> Self {
        ParseErr {
            message: "serde_json error",
        }
    }
}

impl From<std::num::ParseFloatError> for ParseErr {
    fn from(error: std::num::ParseFloatError) -> Self {
        let _: ParseFloatError = error;
        ParseErr {
            message: "parse float error",
        }
    }
}
