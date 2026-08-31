use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Factory is not accepting new registrations")]
    RegistrationsClosed {},

    #[error("Issuer {address} not found")]
    IssuerNotFound { address: String },

    #[error("Issuer {address} is not approved — current status: {status}")]
    IssuerNotApproved { address: String, status: String },

    #[error("Issuer {address} is already registered")]
    IssuerAlreadyRegistered { address: String },

    #[error("Deployment {id} not found")]
    DeploymentNotFound { id: u64 },

    #[error("Contract {addr} not found in registry")]
    ContractNotFound { addr: String },

    #[error("Reserve URI is required before deploying")]
    MissingReserveUri {},

    #[error("Symbol {symbol} is already taken")]
    SymbolTaken { symbol: String },

    #[error("Invalid basis points value {bps} — max 10000")]
    InvalidBps { bps: u64 },

    #[error("Invalid address: {msg}")]
    InvalidAddress { msg: String },

    #[error("Zero amount")]
    ZeroAmount {},

    #[error("Instantiation failed: {msg}")]
    InstantiationFailed { msg: String },

    #[error("Only the registered contract can report its own events")]
    UnauthorizedCallback {},
}
