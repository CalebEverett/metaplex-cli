use base64::DecodeError;
use glob;
use reqwest;
use ring::error::{KeyRejected, Unspecified};
use serde_json;
use std::string::FromUtf8Error;
use thiserror::Error;
use url::ParseError;

#[derive(Error, Debug)]
pub enum ArweaveError {
    #[error("invalid proof")]
    InvalidProof,
    #[error("tags could not be parsed to slices")]
    InvalidTags,
    #[error("transaction is not signed")]
    UnsignedTransaction,
    #[error("file path not provided")]
    MissingFilePath,
    #[error("hashing failed")]
    InvalidHash,
    #[error("unhandled boxed dyn error {0}")]
    BoxedDynStd(#[from] Box<dyn std::error::Error>),
    #[error("io: {0}")]
    IOError(#[from] std::io::Error),
    #[error("key rejected: {0}")]
    KeyRejected(#[from] KeyRejected),
    #[error("unspecified: {0}")]
    Unspecified(#[from] Unspecified),
    #[error("from utf8: {0}")]
    FromUtf8(#[from] FromUtf8Error),
    #[error("base64 decode: {0}")]
    Base64Decode(#[from] DecodeError),
    #[error("serde json: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("url parse error: {0}")]
    UrlParse(#[from] ParseError),
    #[error("reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("glob patters: {0}")]
    GlobPattern(#[from] glob::PatternError),
}
