use cosmwasm_std::Uint128;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::AcceptedDenom;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub amts_denom: String,
    pub accepted_denoms: Vec<AcceptedDenom>,
    pub floor_price_cents: u64,
    pub ceiling_price_cents: u64,
    pub community_pool_address: String,
    pub dex_contract_address: Option<String>,
    pub fee_collector_address: String,
    pub oracle_address: Option<String>,
    /// Governance address — only this address can call governance msgs
    /// Set to the chain's governance module address at instantiation
    pub governance_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub paused: bool,
    pub total_swaps: u64,
    pub total_amts_dispensed: Uint128,
    pub total_usdc_received: Uint128,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE:  Item<State>  = Item::new("state");
