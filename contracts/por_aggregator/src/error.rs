use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Not an authorized oracle: {address}")]
    NotOracle { address: String },

    #[error("Oracle {address} already authorized")]
    OracleAlreadyExists { address: String },

    #[error("Aggregator is paused")]
    Paused {},

    #[error("Reserve data is stale — last update {age_secs}s ago, threshold {threshold_secs}s")]
    StaleData { age_secs: u64, threshold_secs: u64 },

    #[error("No reserve data available — feed has not been initialized")]
    Uninitialized {},

    #[error("Mint would exceed reserves: circulating {circulating} + mint {mint_amount} > reserve {reserve}")]
    ExceedsReserve {
        circulating: String,
        mint_amount: String,
        reserve: String,
    },

    #[error("Oracle {address} already submitted for current round")]
    AlreadySubmitted { address: String },

    #[error("Invalid address: {msg}")]
    InvalidAddress { msg: String },

    #[error("Zero amount")]
    ZeroAmount {},
}
