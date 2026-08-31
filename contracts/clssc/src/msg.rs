use cosmwasm_std::Uint128;
use cw20::Cw20Coin;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// ── Instantiate ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Token name — e.g. "Meridian Closed-Loop Stablecoin"
    pub name: String,
    /// Token symbol — e.g. "CLSSC"
    pub symbol: String,
    /// Decimals — 6 (matching uamts convention)
    pub decimals: u8,
    /// Admin address — can manage minters, whitelist, freeze
    pub admin: String,
    /// Initial minters: list of (address, label) pairs
    pub initial_minters: Vec<(String, String)>,
    #[serde(default)]
    pub mint_caps: Vec<(String, cosmwasm_std::Uint128)>,
    /// Chainlink Proof of Reserve URI
    pub reserve_uri: String,
    /// Whether to start with whitelist enabled
    pub whitelist_enabled: bool,
    /// Initial whitelist addresses (if whitelist_enabled)
    pub initial_whitelist: Vec<String>,
    /// Initial balances (optional — for testing or bootstrapping)
    pub initial_balances: Vec<Cw20Coin>,
    /// On-chain PoR aggregator contract address (optional — can be set later)
    pub aggregator_addr: Option<String>,
    /// Whether to enforce PoR on mint from day one (default: false until aggregator is live)
    pub por_enforced: Option<bool>,
}

// ── Execute ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // ── CW20 standard ──────────────────────────────────────────────────────
    /// Transfer CLSSC to another address
    Transfer {
        recipient: String,
        amount: Uint128,
    },
    /// Transfer CLSSC on behalf of owner (allowance-based)
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    /// Approve spender allowance
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<cw20::Expiration>,
    },
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<cw20::Expiration>,
    },
    /// Send CLSSC to a contract with a message
    Send {
        contract: String,
        amount: Uint128,
        msg: cosmwasm_std::Binary,
    },
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: cosmwasm_std::Binary,
    },

    // ── CLSSC: Mint (authorized minters only) ──────────────────────────────
    /// Mint CLSSC to a recipient
    /// Only callable by addresses in MINTERS map
    /// Represents payroll deposit or fiat-to-CLSSC conversion
    Mint {
        recipient: String,
        amount: Uint128,
        /// Optional memo — e.g. "payroll-2026-05-15" or "deposit-ref-ABC123"
        memo: Option<String>,
    },

    // ── CLSSC: Burn (holder burns own tokens) ──────────────────────────────
    /// Burn CLSSC from caller's balance
    /// Represents redemption — fiat is released off-chain by Meridian
    Burn {
        amount: Uint128,
        /// Optional memo — e.g. "redemption-to-bank-****1234"
        memo: Option<String>,
    },
    /// Burn CLSSC from another address (allowance-based)
    BurnFrom {
        owner: String,
        amount: Uint128,
        memo: Option<String>,
    },

    // ── CLSSC: Admin operations ────────────────────────────────────────────
    /// Add an authorized minter
    AddMinter {
        address: String,
        label: String,
        /// Optional max lifetime mint cap (None = unlimited)
        cap: Option<Uint128>,
    },
    /// Remove an authorized minter
    RemoveMinter {
        address: String,
    },
    /// Add address to transfer whitelist
    AddToWhitelist {
        address: String,
    },
    /// Remove address from transfer whitelist
    RemoveFromWhitelist {
        address: String,
    },
    /// Enable or disable whitelist enforcement
    SetWhitelistEnabled {
        enabled: bool,
    },
    /// Freeze an account (cannot send, receive, or burn)
    FreezeAccount {
        address: String,
        reason: String,
    },
    /// Unfreeze an account
    UnfreezeAccount {
        address: String,
    },
    /// Set or update the PoR aggregator contract address
    SetAggregator {
        aggregator_addr: String,
    },
    /// Enable or disable PoR enforcement on mint
    SetPorEnforced {
        enforced: bool,
    },
    /// Update the Chainlink Proof of Reserve URI
    UpdateReserveUri {
        uri: String,
    },
    /// Transfer admin role to a new address
    TransferAdmin {
        new_admin: String,
    },
    /// Emergency pause — halts all transfers, mints, and burns
    Pause {},
    /// Resume from pause
    Unpause {},
}

// ── Query ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // ── CW20 standard ──────────────────────────────────────────────────────
    Balance {
        address: String,
    },
    TokenInfo {},
    Allowance {
        owner: String,
        spender: String,
    },
    AllAllowances {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    AllAccounts {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    // ── CLSSC specific ─────────────────────────────────────────────────────
    /// Get contract config including reserve URI, admin, supply stats
    Config {},
    /// Check if address is an authorized minter
    IsMinter {
        address: String,
    },
    /// List all authorized minters
    ListMinters {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Check if address is on whitelist
    IsWhitelisted {
        address: String,
    },
    /// Check if address is frozen
    IsFrozen {
        address: String,
    },
    /// Get reserve stats: total_minted, total_burned, circulating_supply
    ReserveStats {},
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub admin: String,
    pub total_minted: Uint128,
    pub total_burned: Uint128,
    pub circulating_supply: Uint128,
    pub reserve_uri: String,
    pub whitelist_enabled: bool,
    pub paused: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsMinterResponse {
    pub is_minter: bool,
    pub label: Option<String>,
    pub cap: Option<Uint128>,
    pub lifetime_minted: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MinterInfo {
    pub address: String,
    pub label: String,
    pub cap: Option<Uint128>,
    pub lifetime_minted: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListMintersResponse {
    pub minters: Vec<MinterInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReserveStatsResponse {
    pub total_minted: Uint128,
    pub total_burned: Uint128,
    pub circulating_supply: Uint128,
}

// ── Migrate ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
