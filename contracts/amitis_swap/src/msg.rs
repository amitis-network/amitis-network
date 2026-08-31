use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── INSTANTIATE ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// AMTS denom on this chain
    pub amts_denom: String,
    /// Accepted input denoms (IBC USDC, OSMO, ATOM)
    pub accepted_denoms: Vec<AcceptedDenom>,
    /// Floor price in USD cents (e.g. 750 = $7.50)
    pub floor_price_cents: u64,
    /// Ceiling price in USD cents (e.g. 1500 = $15.00)
    pub ceiling_price_cents: u64,
    /// Community pool address — receives 80% of input
    pub community_pool_address: String,
    /// DEX contract address — receives 80% for auto-LP
    pub dex_contract_address: Option<String>,
    /// Validator fee collector — receives 20% of input
    pub fee_collector_address: String,
    /// Oracle contract address for price feed
    pub oracle_address: Option<String>,
    /// Governance address — x/gov module address
    /// Only this address can call governance functions
    pub governance_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AcceptedDenom {
    /// IBC denom string
    pub denom: String,
    /// Human readable name
    pub name: String,
    /// USD cents per 1 base unit (e.g. 1 for uusdc since 1,000,000 uusdc = $1.00 = 100 cents)
    /// Price of 1 base unit in USD micro-cents
    pub usd_micro_cents_per_base: u64,
}

// ── EXECUTE ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Swap input token for AMTS
    /// Funds are sent with the message
    Swap {
        /// Minimum AMTS to receive (slippage protection)
        min_amts_out: Uint128,
    },
    /// Governance only — pause swaps
    Pause {},
    /// Governance only — resume swaps
    Resume {},
    /// Governance only — update ceiling price
    UpdateCeiling {
        new_ceiling_cents: u64,
    },
    /// Governance only — update accepted denoms
    UpdateAcceptedDenoms {
        denoms: Vec<AcceptedDenom>,
    },
    /// Governance only — update fee routing addresses
    UpdateFeeRouting {
        community_pool_address: Option<String>,
        fee_collector_address: Option<String>,
        dex_contract_address: Option<String>,
    },
    /// Governance only — update oracle address
    UpdateOracle {
        oracle_address: Option<String>,
    },
}

// ── QUERY ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get contract config
    Config {},
    /// Get swap quote for given input
    Quote {
        denom: String,
        amount: Uint128,
    },
    /// Get current AMTS reserve
    Reserve {},
    /// Get contract state (paused, price, etc)
    State {},
}

// ── RESPONSES ────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub amts_denom: String,
    pub accepted_denoms: Vec<AcceptedDenom>,
    pub floor_price_cents: u64,
    pub ceiling_price_cents: u64,
    pub community_pool_address: String,
    pub dex_contract_address: Option<String>,
    pub fee_collector_address: String,
    pub oracle_address: Option<String>,
    pub governance_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct QuoteResponse {
    pub input_denom: String,
    pub input_amount: Uint128,
    pub amts_out: Uint128,
    pub rate_cents: u64,
    pub fee_amount: Uint128,
    pub rebate_amount: Uint128,
    pub net_fee: Uint128,
    pub community_pool_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReserveResponse {
    pub amts_balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub paused: bool,
    pub current_rate_cents: u64,
    pub floor_price_cents: u64,
    pub ceiling_price_cents: u64,
    pub total_swaps: u64,
    pub total_amts_dispensed: Uint128,
    pub total_usdc_received: Uint128,
}
