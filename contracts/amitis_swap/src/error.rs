use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Contract is paused")]
    Paused {},

    #[error("Unauthorized: only governance can call this")]
    Unauthorized {},

    #[error("No funds sent")]
    NoFunds {},

    #[error("Multiple coins sent, expected one")]
    MultipleFunds {},

    #[error("Unsupported denom: {denom}")]
    UnsupportedDenom { denom: String },

    #[error("Insufficient AMTS reserve: need {need}, have {have}")]
    InsufficientReserve { need: String, have: String },

    #[error("Slippage exceeded: expected at least {min}, got {got}")]
    SlippageExceeded { min: String, got: String },

    #[error("Price out of bounds: {price} cents (floor: {floor}, ceiling: {ceiling})")]
    PriceOutOfBounds { price: u64, floor: u64, ceiling: u64 },

    #[error("Zero input amount")]
    ZeroAmount {},

    #[error("Arithmetic overflow")]
    Overflow {},
}
