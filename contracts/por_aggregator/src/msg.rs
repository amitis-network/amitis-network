use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: String,
    pub factory: Option<String>,
    /// Staleness threshold in seconds (default: 86400 = 24h)
    pub staleness_threshold_secs: Option<u64>,
    /// Minimum oracle submissions per round (default: 1)
    pub min_submissions: Option<u64>,
    pub description: String,
    pub denomination: String,
    /// Initial oracle relayers: [(address, label)]
    pub initial_oracles: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Oracle relayer submits a new reserve attestation
    /// Callable only by addresses in ORACLES map
    SubmitReserve {
        /// Attested reserve amount in denomination units (6 decimals)
        amount: Uint128,
        /// External reference — e.g. bank statement ID, tx hash, report ID
        reference: Option<String>,
    },

    /// Admin: add an authorized oracle relayer
    AddOracle {
        address: String,
        label: String,
    },

    /// Admin: remove an oracle relayer
    RemoveOracle {
        address: String,
    },

    /// Admin: update config
    UpdateConfig {
        staleness_threshold_secs: Option<u64>,
        min_submissions: Option<u64>,
        factory: Option<String>,
    },

    /// Admin: manually override reserve (emergency use — emits a warning event)
    AdminOverride {
        amount: Uint128,
        reason: String,
    },

    /// Admin: pause this aggregator
    Pause {},

    /// Admin: unpause
    Unpause {},

    /// Admin: transfer admin role
    TransferAdmin { new_admin: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the latest round data
    LatestRound {},

    /// Get a specific round by ID
    Round { round_id: u64 },

    /// Get config
    Config {},

    /// Check if the current reserve data is valid for minting
    /// Returns: { valid: bool, reserve: Uint128, reason: Option<String> }
    MintCheck {
        /// Proposed mint amount
        mint_amount: Uint128,
        /// Current circulating supply
        circulating_supply: Uint128,
    },

    /// List authorized oracles
    ListOracles {},

    /// Get the last N rounds of history
    RoundHistory { limit: Option<u32> },
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintCheckResponse {
    /// Whether the mint is allowed
    pub valid: bool,
    /// Current attested reserve amount
    pub reserve: Uint128,
    /// Current circulating supply that was checked against
    pub circulating_supply: Uint128,
    /// Proposed mint amount
    pub mint_amount: Uint128,
    /// Remaining mintable amount (reserve - circulating - mint_amount)
    pub headroom: Uint128,
    /// Reason for rejection if valid == false
    pub reason: Option<String>,
    /// Seconds since last oracle update
    pub data_age_secs: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleInfo {
    pub address: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListOraclesResponse {
    pub oracles: Vec<OracleInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
