use thiserror::Error;

use crate::rule;

#[derive(Debug, Error)]
pub enum Error {
    #[error("url parsing error: {0}")]
    Url(#[from] url::ParseError),
    #[error("request error: {0}")]
    Ureq(#[from] Box<ureq::Error>),
    #[error("json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Need auth")]
    Reauthenticate,
    #[error("no requests remaining")]
    OutOfRequests,
    #[error("Other error: {0}")]
    Other(String),
    #[error("Missing header: {0}")]
    MissingHeader(String),
    #[error("parse error: {0}")]
    ParseFloat(#[from] std::num::ParseFloatError),
    #[error("parse float error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("rule error: {0}")]
    Rule(#[from] rule::Error),
}