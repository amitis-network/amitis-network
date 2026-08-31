use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::state::ReserveType;

// ── Instantiate ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: String,
    /// Code ID of the CLSSC template contract
    pub clssc_code_id: u64,
    /// Mint fee in basis points (10 = 0.10%)
    pub mint_fee_bps: u64,
    /// Burn fee in basis points (10 = 0.10%)
    pub burn_fee_bps: u64,
    /// Address that collects Meridian fees
    pub fee_collector: String,
}

// ── Execute ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // ── Issuer registration ────────────────────────────────────────────────
    /// Anyone can register — triggers KYC flow off-chain
    RegisterIssuer {
        legal_name: String,
        jurisdiction: String,
        /// KYC reference ID from Stripe Identity / Persona
        kyc_ref: String,
        reserve_type: ReserveType,
        reserve_uri: String,
    },

    // ── Admin: issuer management ───────────────────────────────────────────
    ApproveIssuer {
        address: String,
    },
    SuspendIssuer {
        address: String,
        reason: String,
    },
    RevokeIssuer {
        address: String,
        reason: String,
    },
    ReinstateIssuer {
        address: String,
    },

    // ── Issuer: deploy a stablecoin ────────────────────────────────────────
    /// Approved issuers call this to launch their stablecoin
    DeployStablecoin {
        name: String,
        symbol: String,
        decimals: u8,
        /// e.g. "USD", "EUR", "GBP"
        peg_currency: String,
        reserve_type: ReserveType,
        reserve_uri: String,
        /// Whether to start with whitelist enabled
        whitelist_enabled: bool,
        /// Initial whitelisted addresses
        initial_whitelist: Vec<String>,
    },

    // ── Fee callbacks (called by issuer contracts) ─────────────────────────
    /// Called by an issuer's stablecoin contract on every mint
    /// Reports mint volume so factory can collect fee and update stats
    ReportMint {
        issuer: String,
        recipient: String,
        amount: Uint128,
        memo: Option<String>,
    },

    /// Called by an issuer's stablecoin contract on every burn
    ReportBurn {
        issuer: String,
        burner: String,
        amount: Uint128,
        memo: Option<String>,
    },

    // ── Admin: contract management ─────────────────────────────────────────
    /// Pause a specific issuer contract globally
    PauseContract {
        contract_addr: String,
    },
    /// Resume a specific issuer contract
    UnpauseContract {
        contract_addr: String,
    },
    /// Update factory config
    UpdateConfig {
        mint_fee_bps: Option<u64>,
        burn_fee_bps: Option<u64>,
        fee_collector: Option<String>,
        accepting_registrations: Option<bool>,
        clssc_code_id: Option<u64>,
    },
    /// Add or update an approved reserve asset
    SetApprovedAsset {
        id: String,
        name: String,
        approved: bool,
    },
    /// Transfer admin
    TransferAdmin {
        new_admin: String,
    },
}

// ── Query ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Issuer { address: String },
    ListIssuers {
        status_filter: Option<String>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    Deployment { id: u64 },
    DeploymentByContract { contract_addr: String },
    ListDeployments {
        issuer_filter: Option<String>,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    ApprovedAssets {},
    RegistryStats {},
    FeeInfo {},
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RegistryStatsResponse {
    pub total_issuers: u64,
    pub approved_issuers: u64,
    pub total_deployments: u64,
    pub total_fees_collected: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeeInfoResponse {
    pub mint_fee_bps: u64,
    pub burn_fee_bps: u64,
    pub fee_collector: String,
    pub total_fees_collected: Uint128,
}

// ── Migrate ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
