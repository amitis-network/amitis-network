use schemars::JsonSchema;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Meridian admin — can approve issuers, update rules, collect fees
    pub admin: Addr,
    /// CLSSC template code ID — what gets instantiated per issuer
    pub clssc_code_id: u64,
    /// Fee in basis points on every mint (e.g. 10 = 0.10%)
    pub mint_fee_bps: u64,
    /// Fee in basis points on every burn (e.g. 10 = 0.10%)
    pub burn_fee_bps: u64,
    /// Address that receives Meridian's fee cut
    pub fee_collector: Addr,
    /// Total fees collected lifetime (in uamts equivalent)
    pub total_fees_collected: Uint128,
    /// Total stablecoins deployed
    pub total_deployments: u64,
    /// Whether factory is accepting new registrations
    pub accepting_registrations: bool,
}

// ── Issuer ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum IssuerStatus {
    /// KYC submitted, awaiting approval
    Pending,
    /// Approved — can deploy stablecoins
    Approved,
    /// Suspended — existing contracts frozen
    Suspended,
    /// Revoked — permanently barred
    Revoked,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ReserveType {
    /// USD fiat held at a bank
    USDFiat,
    /// US Treasury bills
    TBills,
    /// USDC on-chain
    USDC,
    /// USDT on-chain
    USDT,
    /// Mixed approved assets
    Mixed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IssuerRecord {
    /// Issuer's Amitis wallet address
    pub address: Addr,
    /// Legal name of the issuer
    pub legal_name: String,
    /// Jurisdiction of incorporation
    pub jurisdiction: String,
    /// KYC/KYB reference ID from the verification provider
    pub kyc_ref: String,
    /// Current status
    pub status: IssuerStatus,
    /// Reserve type they declared
    pub reserve_type: ReserveType,
    /// Chainlink PoR feed URI (required before minting is enabled)
    pub reserve_uri: String,
    /// When they registered (block height)
    pub registered_at: u64,
    /// When approved (block height)
    pub approved_at: Option<u64>,
    /// Total stablecoins deployed by this issuer
    pub deployments: u64,
    /// Total volume minted across all their stablecoins (micro units)
    pub lifetime_minted: Uint128,
    /// Total volume burned
    pub lifetime_burned: Uint128,
    /// Total fees paid to Meridian
    pub lifetime_fees: Uint128,
}

// ── Stablecoin deployment record ──────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DeploymentRecord {
    /// Unique deployment ID
    pub id: u64,
    /// Issuer address
    pub issuer: Addr,
    /// Deployed contract address
    pub contract_addr: Addr,
    /// Token name
    pub name: String,
    /// Token symbol
    pub symbol: String,
    /// Decimals
    pub decimals: u8,
    /// Peg currency (e.g. "USD", "EUR", "GBP")
    pub peg_currency: String,
    /// Reserve type
    pub reserve_type: ReserveType,
    /// Chainlink PoR URI
    pub reserve_uri: String,
    /// Block height deployed
    pub deployed_at: u64,
    /// Whether the contract is currently active
    pub active: bool,
    /// Current circulating supply (updated on mint/burn callbacks)
    pub circulating_supply: Uint128,
    /// Lifetime mint volume
    pub lifetime_minted: Uint128,
    /// Lifetime burn volume
    pub lifetime_burned: Uint128,
    /// Lifetime fees paid to Meridian
    pub lifetime_fees: Uint128,
}

// ── Approved reserve assets ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ApprovedAsset {
    /// Asset identifier (e.g. "USDC", "USD_FIAT", "T_BILLS")
    pub id: String,
    /// Human readable name
    pub name: String,
    /// Whether currently approved
    pub approved: bool,
}

// ── Storage ───────────────────────────────────────────────────────────────────

pub const CONFIG: Item<Config> = Item::new("config");

/// Issuers: address → IssuerRecord
pub const ISSUERS: Map<&Addr, IssuerRecord> = Map::new("issuers");

/// Deployments: deployment_id → DeploymentRecord
pub const DEPLOYMENTS: Map<u64, DeploymentRecord> = Map::new("deployments");

/// Contract addr → deployment_id (reverse lookup)
pub const CONTRACT_TO_DEPLOYMENT: Map<&Addr, u64> = Map::new("c2d");

/// Issuer address → list of deployment IDs
pub const ISSUER_DEPLOYMENTS: Map<(&Addr, u64), bool> = Map::new("id");

/// Approved reserve assets
pub const APPROVED_ASSETS: Map<&str, ApprovedAsset> = Map::new("assets");

/// Sequential deployment ID counter
pub const NEXT_DEPLOYMENT_ID: Item<u64> = Item::new("next_id");
