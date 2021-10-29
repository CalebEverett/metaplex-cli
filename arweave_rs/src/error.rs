use thiserror::Error;

#[derive(Error, Debug, Copy, Clone, PartialEq)]
pub enum ArweaveError {
    #[error("Invalid Proof")]
    InvalidProof,
    #[error("Tags could not be parsed to slices.")]
    InvalidTags,
    #[error("Transaction is not signed.")]
    UnsignedTransaction,
    #[error("File path not provided.")]
    MissingFilePath,
}
