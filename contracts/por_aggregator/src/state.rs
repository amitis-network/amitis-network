use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ReserveStatus {
    /// Feed is live and fresh
    Active,
    /// Feed has not updated within staleness_threshold — minting blocked
    Stale,
    /// Admin has manually paused this feed
    Paused,
    /// Feed has never received a value
    Uninitialized,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    /// Admin — can add/remove oracles, set thresholds, pause
    pub admin: Addr,
    /// Meridian factory address — can query status for gating deployments
    pub factory: Option<Addr>,
    /// How many seconds before a feed is considered stale (default: 86400 = 24h)
    pub staleness_threshold_secs: u64,
    /// Minimum number of oracle submissions required per round (default: 1)
    pub min_submissions: u64,
    /// Whether this aggregator is paused globally
    pub paused: bool,
    /// Human-readable description of what this aggregator tracks
    pub description: String,
    /// Currency denomination of the reserve value (e.g. "USD", "USDC")
    pub denomination: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RoundData {
    /// Sequential round ID
    pub round_id: u64,
    /// Attested reserve amount (in smallest denomination units — e.g. micro-USD = 6 decimals)
    pub reserve_amount: Uint128,
    /// Block timestamp when this round was started
    pub started_at: u64,
    /// Block timestamp when this round was finalized
    pub updated_at: u64,
    /// Block height of the update
    pub block_height: u64,
    /// Number of oracle submissions in this round
    pub submission_count: u64,
    /// Status of this round's data
    pub status: ReserveStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OracleSubmission {
    /// Oracle that submitted
    pub oracle: Addr,
    /// Value submitted
    pub value: Uint128,
    /// Timestamp of submission
    pub timestamp: u64,
}

// ── Storage ───────────────────────────────────────────────────────────────────

pub const CONFIG: Item<Config> = Item::new("config");

/// Current round data (latest attested reserve)
pub const LATEST_ROUND: Item<RoundData> = Item::new("latest_round");

/// Round ID counter
pub const ROUND_ID: Item<u64> = Item::new("round_id");

/// Authorized oracle relayers: address → label
pub const ORACLES: Map<&Addr, String> = Map::new("oracles");

/// Pending submissions for current round: oracle_addr → OracleSubmission
pub const PENDING_SUBMISSIONS: Map<&Addr, OracleSubmission> = Map::new("pending");

/// Historical rounds: round_id → RoundData (last 100 rounds retained)
pub const ROUND_HISTORY: Map<u64, RoundData> = Map::new("history");
