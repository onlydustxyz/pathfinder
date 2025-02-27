//! Sequencer related error types.
use crate::rpc::v01::types::reply::ErrorCode as RpcErrorCode;
use jsonrpsee::{core::error::Error, types::error::CallError};
use serde::{Deserialize, Serialize};

/// Sequencer errors.
#[derive(Debug, thiserror::Error)]
pub enum SequencerError {
    /// Starknet specific errors.
    #[error(transparent)]
    StarknetError(#[from] StarknetError),
    /// Errors directly coming from reqwest
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    /// Custom errors that we fidded with because the original error was either
    /// not informative enough or bloated
    #[error("error decoding response body: invalid error variant")]
    InvalidStarknetErrorVariant,
}

impl From<SequencerError> for Error {
    fn from(e: SequencerError) -> Self {
        match e {
            SequencerError::ReqwestError(e) => Error::Call(CallError::Failed(e.into())),
            SequencerError::InvalidStarknetErrorVariant => Error::Call(CallError::Failed(e.into())),
            SequencerError::StarknetError(e) => match e.code {
                StarknetErrorCode::OutOfRangeBlockHash | StarknetErrorCode::BlockNotFound
                    if e.message.contains("Block hash") =>
                {
                    RpcErrorCode::InvalidBlockId.into()
                }
                StarknetErrorCode::OutOfRangeContractAddress
                | StarknetErrorCode::UninitializedContract => RpcErrorCode::ContractNotFound.into(),
                StarknetErrorCode::OutOfRangeTransactionHash => {
                    RpcErrorCode::InvalidTransactionHash.into()
                }
                StarknetErrorCode::TransactionFailed => RpcErrorCode::InvalidCallData.into(),
                StarknetErrorCode::TransactionLimitExceeded => {
                    Error::Call(CallError::Failed(e.into()))
                }
                StarknetErrorCode::EntryPointNotFound => {
                    RpcErrorCode::InvalidMessageSelector.into()
                }
                StarknetErrorCode::BlockNotFound if e.message.contains("Block number") => {
                    RpcErrorCode::InvalidBlockId.into()
                }
                StarknetErrorCode::InvalidContractDefinition => RpcErrorCode::ContractError.into(),
                StarknetErrorCode::BlockNotFound
                | StarknetErrorCode::SchemaValidationError
                | StarknetErrorCode::MalformedRequest
                | StarknetErrorCode::UnsupportedSelectorForFee
                | StarknetErrorCode::OutOfRangeBlockHash
                | StarknetErrorCode::NotPermittedContract
                | StarknetErrorCode::InvalidTransactionNonce
                | StarknetErrorCode::OutOfRangeFee
                | StarknetErrorCode::InvalidTransactionVersion
                | StarknetErrorCode::InvalidProgram => Error::Call(CallError::Failed(e.into())),
                StarknetErrorCode::UndeclaredClass => RpcErrorCode::InvalidContractClassHash.into(),
            },
        }
    }
}

/// Used for deserializing specific Starknet sequencer error data.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StarknetError {
    pub code: StarknetErrorCode,
    pub message: String,
    // The `problems` field is intentionally omitted here
    // Let's deserialize it if it proves necessary
}

impl std::error::Error for StarknetError {}

impl std::fmt::Display for StarknetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Represents starknet specific error codes reported by the sequencer.
#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum StarknetErrorCode {
    #[serde(rename = "StarknetErrorCode.BLOCK_NOT_FOUND")]
    BlockNotFound,
    #[serde(rename = "StarknetErrorCode.ENTRY_POINT_NOT_FOUND_IN_CONTRACT")]
    EntryPointNotFound,
    #[serde(rename = "StarknetErrorCode.OUT_OF_RANGE_CONTRACT_ADDRESS")]
    OutOfRangeContractAddress,
    #[serde(rename = "StarkErrorCode.SCHEMA_VALIDATION_ERROR")]
    SchemaValidationError,
    #[serde(rename = "StarknetErrorCode.TRANSACTION_FAILED")]
    TransactionFailed,
    #[serde(rename = "StarknetErrorCode.UNINITIALIZED_CONTRACT")]
    UninitializedContract,
    #[serde(rename = "StarknetErrorCode.OUT_OF_RANGE_BLOCK_HASH")]
    OutOfRangeBlockHash,
    #[serde(rename = "StarknetErrorCode.OUT_OF_RANGE_TRANSACTION_HASH")]
    OutOfRangeTransactionHash,
    #[serde(rename = "StarkErrorCode.MALFORMED_REQUEST")]
    MalformedRequest,
    #[serde(rename = "StarknetErrorCode.UNSUPPORTED_SELECTOR_FOR_FEE")]
    UnsupportedSelectorForFee,
    #[serde(rename = "StarknetErrorCode.INVALID_CONTRACT_DEFINITION")]
    InvalidContractDefinition,
    #[serde(rename = "StarknetErrorCode.NON_PERMITTED_CONTRACT")]
    NotPermittedContract,
    #[serde(rename = "StarknetErrorCode.UNDECLARED_CLASS")]
    UndeclaredClass,
    /// May be returned by the transaction write api.
    #[serde(rename = "StarknetErrorCode.TRANSACTION_LIMIT_EXCEEDED")]
    TransactionLimitExceeded,
    #[serde(rename = "StarknetErrorCode.INVALID_TRANSACTION_NONCE")]
    InvalidTransactionNonce,
    #[serde(rename = "StarknetErrorCode.OUT_OF_RANGE_FEE")]
    OutOfRangeFee,
    #[serde(rename = "StarknetErrorCode.INVALID_TRANSACTION_VERSION")]
    InvalidTransactionVersion,
    #[serde(rename = "StarknetErrorCode.INVALID_PROGRAM")]
    InvalidProgram,
}
