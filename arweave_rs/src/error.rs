use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum ArweaveError {
    /// Invalid instruction
    #[error("Invalid Proof")]
    InvalidProof,
    #[error("Tags could not be parsed to slices.")]
    InvalidTags,
}
