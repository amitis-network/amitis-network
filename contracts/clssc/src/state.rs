use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

// ── Contract config ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    /// Admin address — can add/remove minters, freeze, update reserve URI
    pub admin: Addr,
    /// Total CLSSC minted (tracks circulating supply for PoR checks)
    pub total_minted: u128,
    /// Total CLSSC burned
    pub total_burned: u128,
    /// URI to Chainlink Proof of Reserve feed (updated by admin)
    pub reserve_uri: String,
    /// On-chain PoR Aggregator contract address
    /// When set, mint guard queries this before every mint
    pub aggregator_addr: Option<Addr>,
    /// Whether to enforce PoR check on mint (can be disabled during setup)
    pub por_enforced: bool,
    /// Whether closed-loop transfer whitelist is active
    pub whitelist_enabled: bool,
    /// Whether the contract is paused (emergency stop)
    pub paused: bool,
}

// ── Storage keys ──────────────────────────────────────────────────────────────

/// Global config
pub const CONFIG: Item<Config> = Item::new("config");

/// Authorized minters: address → label (e.g. "meridian-payroll-processor")
pub const MINTERS: Map<&Addr, String> = Map::new("minters");

/// Transfer whitelist: address → true (when whitelist_enabled)
/// Only whitelisted addresses can send or receive CLSSC
pub const WHITELIST: Map<&Addr, bool> = Map::new("whitelist");

/// Frozen accounts: address → reason
/// Frozen accounts cannot send, receive, or burn
pub const FROZEN: Map<&Addr, String> = Map::new("frozen");

/// Per-address mint cap (optional): address → max lifetime mint amount
/// None = unlimited. Used to cap how much any single minter can issue.
pub const MINT_CAPS: Map<&Addr, u128> = Map::new("mint_caps");

/// Per-address lifetime minted (for cap enforcement)
pub const LIFETIME_MINTED: Map<&Addr, u128> = Map::new("lifetime_minted");
