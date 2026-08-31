// por_onchain — On-Chain Reserve Verifier
//
// For issuers whose reserves are held in CW20 tokens (USDC, USDT, etc.)
// on Amitis Network. This contract queries the issuer's holding wallet balance
// directly from the CW20 contract — no external oracle needed.
//
// The result is posted to the linked por_aggregator contract automatically
// by calling SubmitReserve, or can be queried directly by CLSSC.
//
// Deploy one of these per issuer who holds CW20 reserves.

use cosmwasm_std::{
    entry_point, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Order, QueryRequest, Response, StdResult, Uint128, WasmMsg,
    WasmQuery,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20QueryMsg};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const CONTRACT_NAME: &str = "meridian-por-onchain";
const CONTRACT_VERSION: &str = "0.1.0";

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] cosmwasm_std::StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Invalid address: {msg}")]
    InvalidAddress { msg: String },
    #[error("Reserve source not found: {id}")]
    SourceNotFound { id: String },
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    pub admin: cosmwasm_std::Addr,
    /// The por_aggregator contract this verifier reports to
    pub aggregator_addr: cosmwasm_std::Addr,
    /// Issuer address — whose balances we're tracking
    pub issuer_addr: cosmwasm_std::Addr,
    /// Description
    pub description: String,
}

/// A CW20 token held as reserve
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReserveSource {
    /// Unique ID for this source
    pub id: String,
    /// CW20 contract address
    pub token_contract: String,
    /// Human-readable name (e.g. "USDC on Amitis")
    pub label: String,
    /// Whether this source is active
    pub active: bool,
    /// Last known balance (cached from last snapshot)
    pub last_balance: Uint128,
    /// When last_balance was recorded (block height)
    pub last_snapshot_height: u64,
}

const CONFIG: Item<Config> = Item::new("config");
const SOURCES: Map<&str, ReserveSource> = Map::new("sources");

// ── Messages ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: String,
    pub aggregator_addr: String,
    pub issuer_addr: String,
    pub description: String,
    /// Initial reserve sources
    pub initial_sources: Vec<ReserveSource>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Query all CW20 balances and post total to the aggregator
    /// Callable by anyone — this is a public action
    Snapshot {},
    /// Add a CW20 reserve source
    AddSource { source: ReserveSource },
    /// Deactivate a source
    RemoveSource { id: String },
    /// Update the aggregator address
    UpdateAggregator { aggregator_addr: String },
    /// Transfer admin
    TransferAdmin { new_admin: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get config
    Config {},
    /// Get all reserve sources with current balances
    Sources {},
    /// Get the total reserve across all active sources
    TotalReserve {},
    /// Get a specific source
    Source { id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TotalReserveResponse {
    pub total: Uint128,
    pub sources: Vec<ReserveSourceBalance>,
    pub snapshot_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReserveSourceBalance {
    pub id: String,
    pub label: String,
    pub token_contract: String,
    pub balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

// ── Entry points ──────────────────────────────────────────────────────────────

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    let aggregator = deps.api.addr_validate(&msg.aggregator_addr)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    let issuer = deps.api.addr_validate(&msg.issuer_addr)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;

    CONFIG.save(deps.storage, &Config {
        admin: admin.clone(),
        aggregator_addr: aggregator.clone(),
        issuer_addr: issuer.clone(),
        description: msg.description.clone(),
    })?;

    for source in msg.initial_sources {
        SOURCES.save(deps.storage, &source.id, &source)?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate_por_onchain")
        .add_attribute("admin", admin.to_string())
        .add_attribute("aggregator", aggregator.to_string())
        .add_attribute("issuer", issuer.to_string()))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Snapshot {} =>
            exec_snapshot(deps, env, info),
        ExecuteMsg::AddSource { source } =>
            exec_add_source(deps, info, source),
        ExecuteMsg::RemoveSource { id } =>
            exec_remove_source(deps, info, id),
        ExecuteMsg::UpdateAggregator { aggregator_addr } =>
            exec_update_aggregator(deps, info, aggregator_addr),
        ExecuteMsg::TransferAdmin { new_admin } =>
            exec_transfer_admin(deps, info, new_admin),
    }
}

fn exec_snapshot(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Query all active CW20 balances
    let sources: Vec<ReserveSource> = SOURCES
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .map(|(_, v)| v)
        .filter(|s| s.active)
        .collect();

    let mut total = Uint128::zero();
    let mut updated_sources = vec![];

    for mut source in sources {
        // Query CW20 balance of issuer's wallet at this token contract
        let balance_query = QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: source.token_contract.clone(),
            msg: to_json_binary(&Cw20QueryMsg::Balance {
                address: config.issuer_addr.to_string(),
            })?,
        });

        let balance_resp: BalanceResponse = deps.querier.query(&balance_query)?;
        let balance = balance_resp.balance;

        total += balance;
        source.last_balance = balance;
        source.last_snapshot_height = env.block.height;
        updated_sources.push(source.clone());
        SOURCES.save(deps.storage, &source.id, &source)?;
    }

    // Post total to the aggregator contract
    let submit_msg = WasmMsg::Execute {
        contract_addr: config.aggregator_addr.to_string(),
        msg: to_json_binary(&serde_json::json!({
            "submit_reserve": {
                "amount": total.to_string(),
                "reference": format!("onchain-snapshot-block-{}", env.block.height)
            }
        }))?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(submit_msg))
        .add_attribute("action", "snapshot")
        .add_attribute("total_reserve", total.to_string())
        .add_attribute("sources_queried", updated_sources.len().to_string())
        .add_attribute("block_height", env.block.height.to_string()))
}

fn exec_add_source(
    deps: DepsMut,
    info: MessageInfo,
    source: ReserveSource,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin { return Err(ContractError::Unauthorized {}); }
    SOURCES.save(deps.storage, &source.id, &source)?;
    Ok(Response::new()
        .add_attribute("action", "add_source")
        .add_attribute("id", source.id)
        .add_attribute("token_contract", source.token_contract))
}

fn exec_remove_source(
    deps: DepsMut,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin { return Err(ContractError::Unauthorized {}); }
    let mut source = SOURCES.load(deps.storage, &id)
        .map_err(|_| ContractError::SourceNotFound { id: id.clone() })?;
    source.active = false;
    SOURCES.save(deps.storage, &id, &source)?;
    Ok(Response::new()
        .add_attribute("action", "remove_source")
        .add_attribute("id", id))
}

fn exec_update_aggregator(
    deps: DepsMut,
    info: MessageInfo,
    aggregator_addr: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin { return Err(ContractError::Unauthorized {}); }
    config.aggregator_addr = deps.api.addr_validate(&aggregator_addr)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_aggregator"))
}

fn exec_transfer_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin { return Err(ContractError::Unauthorized {}); }
    config.admin = deps.api.addr_validate(&new_admin)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "transfer_admin"))
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),

        QueryMsg::Sources {} => {
            let sources: Vec<ReserveSource> = SOURCES
                .range(deps.storage, None, None, Order::Ascending)
                .filter_map(|r| r.ok())
                .map(|(_, v)| v)
                .collect();
            to_json_binary(&sources)
        },

        QueryMsg::Source { id } => {
            to_json_binary(&SOURCES.load(deps.storage, &id)?)
        },

        QueryMsg::TotalReserve {} => {
            let config = CONFIG.load(deps.storage)?;
            let sources: Vec<ReserveSource> = SOURCES
                .range(deps.storage, None, None, Order::Ascending)
                .filter_map(|r| r.ok())
                .map(|(_, v)| v)
                .filter(|s| s.active)
                .collect();

            let mut total = Uint128::zero();
            let mut balances = vec![];

            for source in &sources {
                let balance_query = QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: source.token_contract.clone(),
                    msg: to_json_binary(&Cw20QueryMsg::Balance {
                        address: config.issuer_addr.to_string(),
                    })?,
                });
                let resp: BalanceResponse = deps.querier.query(&balance_query)?;
                total += resp.balance;
                balances.push(ReserveSourceBalance {
                    id: source.id.clone(),
                    label: source.label.clone(),
                    token_contract: source.token_contract.clone(),
                    balance: resp.balance,
                });
            }

            to_json_binary(&TotalReserveResponse {
                total,
                sources: balances,
                snapshot_height: env.block.height,
            })
        },
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
}
