use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: caller is not admin")]
    Unauthorized {},

    #[error("Unauthorized: caller is not an authorized minter")]
    NotMinter {},

    #[error("Contract is paused")]
    Paused {},

    #[error("Account {address} is frozen: {reason}")]
    AccountFrozen { address: String, reason: String },

    #[error("Transfer not allowed: address {address} is not on the whitelist")]
    NotWhitelisted { address: String },

    #[error("Insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: String, need: String },

    #[error("Mint cap exceeded: minter {minter} has cap {cap}, already minted {minted}, requested {requested}")]
    MintCapExceeded {
        minter: String,
        cap: String,
        minted: String,
        requested: String,
    },

    #[error("Address {address} is already a minter")]
    AlreadyMinter { address: String },

    #[error("Address {address} is not a minter")]
    NotAMinter { address: String },

    #[error("Address {address} is already whitelisted")]
    AlreadyWhitelisted { address: String },

    #[error("Address {address} is not whitelisted")]
    NotInWhitelist { address: String },

    #[error("Address {address} is already frozen")]
    AlreadyFrozen { address: String },

    #[error("Address {address} is not frozen")]
    NotFrozen { address: String },

    #[error("Invalid address: {msg}")]
    InvalidAddress { msg: String },

    #[error("Zero amount not allowed")]
    ZeroAmount {},

    #[error("Overflow in arithmetic operation")]
    Overflow {},
}
