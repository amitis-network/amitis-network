use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, Order,
};
use cw2::set_contract_version;
use cw20_base::contract as cw20;
use cw20_base::state::{TOKEN_INFO, BALANCES, ALLOWANCES};
use cw20_base::msg::{InstantiateMsg as Cw20InstantiateMsg};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, MigrateMsg,
    ConfigResponse, IsMinterResponse, MinterInfo, ListMintersResponse,
    ReserveStatsResponse,
};
use crate::state::{
    Config, CONFIG, MINTERS, WHITELIST, FROZEN,
    MINT_CAPS, LIFETIME_MINTED,
};

const CONTRACT_NAME: &str = "crates.io:clssc";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Instantiate ───────────────────────────────────────────────────────────────

pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;

    // Init CW20 base token
    let cw20_init = Cw20InstantiateMsg {
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        decimals: msg.decimals,
        initial_balances: msg.initial_balances.clone(),
        mint: None,
        marketing: None,
    };
    cw20::instantiate(deps.branch(), env.clone(), info.clone(), cw20_init)
        .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))?;

    // Store CLSSC config
    CONFIG.save(deps.storage, &Config {
        admin: admin.clone(),
        total_minted: 0u128,
        total_burned: 0u128,
        reserve_uri: msg.reserve_uri.clone(),
        aggregator_addr: None, // set after deploy via SetAggregator
        por_enforced: msg.por_enforced.unwrap_or(false),
        whitelist_enabled: msg.whitelist_enabled,
        paused: false,
    })?;

    // Register initial minters
    for (addr_str, label) in &msg.initial_minters {
        let addr = deps.api.addr_validate(addr_str)
            .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
        MINTERS.save(deps.storage, &addr, label)?;
    }

    // Apply mint caps
    for (addr_str, cap) in &msg.mint_caps {
        let addr = deps.api.addr_validate(addr_str)
            .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
        MINT_CAPS.save(deps.storage, &addr, &cap.u128())?;
    }

    // Register initial whitelist
    for addr_str in &msg.initial_whitelist {
        let addr = deps.api.addr_validate(addr_str)
            .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
        WHITELIST.save(deps.storage, &addr, &true)?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate_clssc")
        .add_attribute("admin", admin.to_string())
        .add_attribute("name", msg.name)
        .add_attribute("symbol", msg.symbol)
        .add_attribute("reserve_uri", msg.reserve_uri)
        .add_attribute("whitelist_enabled", msg.whitelist_enabled.to_string()))
}

// ── Execute ───────────────────────────────────────────────────────────────────

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Pause check — only admin ops bypass pause
    let is_admin_op = matches!(msg,
        ExecuteMsg::Pause {} |
        ExecuteMsg::Unpause {} |
        ExecuteMsg::FreezeAccount { .. } |
        ExecuteMsg::UnfreezeAccount { .. } |
        ExecuteMsg::AddMinter { .. } |
        ExecuteMsg::RemoveMinter { .. } |
        ExecuteMsg::TransferAdmin { .. } |
        ExecuteMsg::UpdateReserveUri { .. } |
        ExecuteMsg::SetWhitelistEnabled { .. } |
        ExecuteMsg::AddToWhitelist { .. } |
        ExecuteMsg::RemoveFromWhitelist { .. }
    );

    if config.paused && !is_admin_op {
        return Err(ContractError::Paused {});
    }

    match msg {
        // ── CW20 standard ─────────────────────────────────────────────────
        ExecuteMsg::Transfer { recipient, amount } =>
            exec_transfer(deps, env, info, recipient, amount),

        ExecuteMsg::TransferFrom { owner, recipient, amount } =>
            exec_transfer_from(deps, env, info, owner, recipient, amount),

        ExecuteMsg::IncreaseAllowance { spender, amount, expires } => {
            cw20_base::allowances::execute_increase_allowance(deps, env, info, spender, amount, expires)
                .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))
        },

        ExecuteMsg::DecreaseAllowance { spender, amount, expires } => {
            cw20_base::allowances::execute_decrease_allowance(deps, env, info, spender, amount, expires)
                .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))
        },

        ExecuteMsg::Send { contract, amount, msg } =>
            exec_send(deps, env, info, contract, amount, msg),

        ExecuteMsg::SendFrom { owner, contract, amount, msg } =>
            exec_send_from(deps, env, info, owner, contract, amount, msg),

        // ── Mint / Burn ───────────────────────────────────────────────────
        ExecuteMsg::Mint { recipient, amount, memo } =>
            exec_mint(deps, env, info, recipient, amount, memo),

        ExecuteMsg::Burn { amount, memo } =>
            exec_burn(deps, env, info, amount, memo),

        ExecuteMsg::BurnFrom { owner, amount, memo } =>
            exec_burn_from(deps, env, info, owner, amount, memo),

        // ── Admin ─────────────────────────────────────────────────────────
        ExecuteMsg::AddMinter { address, label, cap } =>
            exec_add_minter(deps, info, address, label, cap),

        ExecuteMsg::RemoveMinter { address } =>
            exec_remove_minter(deps, info, address),

        ExecuteMsg::AddToWhitelist { address } =>
            exec_add_whitelist(deps, info, address),

        ExecuteMsg::RemoveFromWhitelist { address } =>
            exec_remove_whitelist(deps, info, address),

        ExecuteMsg::SetWhitelistEnabled { enabled } =>
            exec_set_whitelist_enabled(deps, info, enabled),

        ExecuteMsg::FreezeAccount { address, reason } =>
            exec_freeze(deps, info, address, reason),

        ExecuteMsg::UnfreezeAccount { address } =>
            exec_unfreeze(deps, info, address),

        ExecuteMsg::SetAggregator { aggregator_addr } =>
            exec_set_aggregator(deps, info, aggregator_addr),

        ExecuteMsg::SetPorEnforced { enforced } =>
            exec_set_por_enforced(deps, info, enforced),

        ExecuteMsg::UpdateReserveUri { uri } =>
            exec_update_reserve_uri(deps, info, uri),

        ExecuteMsg::TransferAdmin { new_admin } =>
            exec_transfer_admin(deps, info, new_admin),

        ExecuteMsg::Pause {} => exec_pause(deps, info),
        ExecuteMsg::Unpause {} => exec_unpause(deps, info),
    }
}

// ── Transfer helpers ──────────────────────────────────────────────────────────

fn check_transfer_allowed(
    deps: &Deps,
    config: &Config,
    from: &Addr,
    to: &Addr,
) -> Result<(), ContractError> {
    // Frozen check
    if let Ok(Some(reason)) = FROZEN.may_load(deps.storage, from).map(|r| r) {
        return Err(ContractError::AccountFrozen {
            address: from.to_string(),
            reason,
        });
    }
    if let Ok(Some(reason)) = FROZEN.may_load(deps.storage, to).map(|r| r) {
        return Err(ContractError::AccountFrozen {
            address: to.to_string(),
            reason,
        });
    }

    // Whitelist check
    if config.whitelist_enabled {
        let from_ok = WHITELIST.may_load(deps.storage, from)?.unwrap_or(false);
        if !from_ok {
            return Err(ContractError::NotWhitelisted { address: from.to_string() });
        }
        let to_ok = WHITELIST.may_load(deps.storage, to)?.unwrap_or(false);
        if !to_ok {
            return Err(ContractError::NotWhitelisted { address: to.to_string() });
        }
    }

    Ok(())
}

fn exec_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() { return Err(ContractError::ZeroAmount {}); }
    let config = CONFIG.load(deps.storage)?;
    let rcpt = deps.api.addr_validate(&recipient)?;
    check_transfer_allowed(&deps.as_ref(), &config, &info.sender, &rcpt)?;
    cw20::execute_transfer(deps, env, info, recipient, amount)
        .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))
}

fn exec_transfer_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() { return Err(ContractError::ZeroAmount {}); }
    let config = CONFIG.load(deps.storage)?;
    let owner_addr = deps.api.addr_validate(&owner)?;
    let rcpt = deps.api.addr_validate(&recipient)?;
    check_transfer_allowed(&deps.as_ref(), &config, &owner_addr, &rcpt)?;
    cw20_base::allowances::execute_transfer_from(deps, env, info, owner, recipient, amount)
        .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))
}

fn exec_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    if amount.is_zero() { return Err(ContractError::ZeroAmount {}); }
    let config = CONFIG.load(deps.storage)?;
    let contract_addr = deps.api.addr_validate(&contract)?;
    check_transfer_allowed(&deps.as_ref(), &config, &info.sender, &contract_addr)?;
    cw20::execute_send(deps, env, info, contract, amount, msg)
        .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))
}

fn exec_send_from(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    if amount.is_zero() { return Err(ContractError::ZeroAmount {}); }
    let config = CONFIG.load(deps.storage)?;
    let owner_addr = deps.api.addr_validate(&owner)?;
    let contract_addr = deps.api.addr_validate(&contract)?;
    check_transfer_allowed(&deps.as_ref(), &config, &owner_addr, &contract_addr)?;
    cw20_base::allowances::execute_send_from(deps.branch(), env, info, owner, contract, amount, msg)
        .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))
}

// ── Mint ──────────────────────────────────────────────────────────────────────

fn exec_mint(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
    memo: Option<String>,
) -> Result<Response, ContractError> {
    if amount.is_zero() { return Err(ContractError::ZeroAmount {}); }

    // Must be authorized minter
    let label = MINTERS.may_load(deps.storage, &info.sender)?
        .ok_or(ContractError::NotMinter {})?;

    let rcpt = deps.api.addr_validate(&recipient)?;

    // Frozen check on recipient
    let config = CONFIG.load(deps.storage)?;
    if let Some(reason) = FROZEN.may_load(deps.storage, &rcpt)? {
        return Err(ContractError::AccountFrozen { address: rcpt.to_string(), reason });
    }

    // Whitelist check on recipient (if enabled)
    if config.whitelist_enabled {
        let ok = WHITELIST.may_load(deps.storage, &rcpt)?.unwrap_or(false);
        if !ok {
            return Err(ContractError::NotWhitelisted { address: rcpt.to_string() });
        }
    }

    // ── PoR Guard — query aggregator before minting ───────────────────────
    if config.por_enforced {
        if let Some(ref aggregator) = config.aggregator_addr {
            // Get current circulating supply
            let token_info = TOKEN_INFO.load(deps.storage)?;
            let circulating = token_info.total_supply;

            // Query MintCheck on the aggregator
            let check_query = cosmwasm_std::QueryRequest::Wasm(
                cosmwasm_std::WasmQuery::Smart {
                    contract_addr: aggregator.to_string(),
                    msg: to_json_binary(&serde_json::json!({
                        "mint_check": {
                            "mint_amount": amount.to_string(),
                            "circulating_supply": circulating.to_string()
                        }
                    }))?,
                }
            );

            #[derive(serde::Deserialize)]
            struct MintCheckResp { valid: bool, reason: Option<String> }

            let check: MintCheckResp = deps.querier.query(&check_query)
                .map_err(|e| ContractError::Std(e))?;

            if !check.valid {
                return Err(ContractError::Std(
                    cosmwasm_std::StdError::generic_err(format!(
                        "PoR check failed: {}",
                        check.reason.unwrap_or_else(|| "reserve insufficient or stale".to_string())
                    ))
                ));
            }
        } else {
            // por_enforced=true but no aggregator set — block mint
            return Err(ContractError::Std(
                cosmwasm_std::StdError::generic_err(
                    "PoR enforcement enabled but no aggregator configured"
                )
            ));
        }
    }

    // Mint cap check
    if let Some(cap) = MINT_CAPS.may_load(deps.storage, &info.sender)? {
        let already = LIFETIME_MINTED.may_load(deps.storage, &info.sender)?.unwrap_or(0u128);
        let requested = amount.u128();
        if already + requested > cap {
            return Err(ContractError::MintCapExceeded {
                minter: info.sender.to_string(),
                cap: cap.to_string(),
                minted: already.to_string(),
                requested: requested.to_string(),
            });
        }
        LIFETIME_MINTED.save(deps.storage, &info.sender, &(already + requested))?;
    }

    // Perform the mint via CW20 base
    // CW20 base minter is set to the contract itself — we call directly
    let mut balance = BALANCES.may_load(deps.storage, &rcpt)?.unwrap_or(Uint128::zero());
    balance += amount;
    BALANCES.save(deps.storage, &rcpt, &balance)?;

    // Update token info total supply
    TOKEN_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_supply += amount;
        Ok(info)
    })?;

    // Update CLSSC config totals
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.total_minted += amount.u128();
        Ok(cfg)
    })?;

    let mut resp = Response::new()
        .add_attribute("action", "clssc_mint")
        .add_attribute("minter", info.sender.to_string())
        .add_attribute("minter_label", label)
        .add_attribute("recipient", rcpt.to_string())
        .add_attribute("amount", amount.to_string());

    if let Some(m) = memo {
        resp = resp.add_attribute("memo", m);
    }

    Ok(resp)
}

// ── Burn ──────────────────────────────────────────────────────────────────────

fn exec_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    memo: Option<String>,
) -> Result<Response, ContractError> {
    if amount.is_zero() { return Err(ContractError::ZeroAmount {}); }

    // Frozen check
    if let Some(reason) = FROZEN.may_load(deps.storage, &info.sender)? {
        return Err(ContractError::AccountFrozen { address: info.sender.to_string(), reason });
    }

    // Deduct from balance
    BALANCES.update(deps.storage, &info.sender, |bal| -> StdResult<_> {
        Ok(bal.unwrap_or_default().checked_sub(amount)?)
    })?;

    // Reduce total supply
    TOKEN_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_supply = info.total_supply.checked_sub(amount)?;
        Ok(info)
    })?;

    // Update burn counter
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.total_burned += amount.u128();
        Ok(cfg)
    })?;

    let mut resp = Response::new()
        .add_attribute("action", "clssc_burn")
        .add_attribute("burner", info.sender.to_string())
        .add_attribute("amount", amount.to_string());

    if let Some(m) = memo {
        resp = resp.add_attribute("memo", m);
    }

    Ok(resp)
}

fn exec_burn_from(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    amount: Uint128,
    memo: Option<String>,
) -> Result<Response, ContractError> {
    if amount.is_zero() { return Err(ContractError::ZeroAmount {}); }
    let owner_addr = deps.api.addr_validate(&owner)?;

    // Frozen check on owner
    if let Some(reason) = FROZEN.may_load(deps.storage, &owner_addr)? {
        return Err(ContractError::AccountFrozen { address: owner.clone(), reason });
    }

    // Use CW20 allowance deduction
    cw20_base::allowances::execute_burn_from(deps.branch(), env, info.clone(), owner, amount)
        .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))?;

    // Update burn counter
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.total_burned += amount.u128();
        Ok(cfg)
    })?;

    let mut resp = Response::new()
        .add_attribute("action", "clssc_burn_from")
        .add_attribute("burner", info.sender.to_string())
        .add_attribute("owner", owner_addr.to_string())
        .add_attribute("amount", amount.to_string());

    if let Some(m) = memo {
        resp = resp.add_attribute("memo", m);
    }

    Ok(resp)
}

// ── Admin operations ──────────────────────────────────────────────────────────

fn only_admin(deps: &Deps, sender: &Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if *sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

fn exec_add_minter(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    label: String,
    cap: Option<Uint128>,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    if MINTERS.may_load(deps.storage, &addr)?.is_some() {
        return Err(ContractError::AlreadyMinter { address });
    }
    MINTERS.save(deps.storage, &addr, &label)?;
    if let Some(c) = cap {
        MINT_CAPS.save(deps.storage, &addr, &c.u128())?;
    }
    Ok(Response::new()
        .add_attribute("action", "add_minter")
        .add_attribute("address", addr.to_string())
        .add_attribute("label", label))
}

fn exec_remove_minter(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    if MINTERS.may_load(deps.storage, &addr)?.is_none() {
        return Err(ContractError::NotAMinter { address });
    }
    MINTERS.remove(deps.storage, &addr);
    MINT_CAPS.remove(deps.storage, &addr);
    Ok(Response::new()
        .add_attribute("action", "remove_minter")
        .add_attribute("address", addr.to_string()))
}

fn exec_add_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    WHITELIST.save(deps.storage, &addr, &true)?;
    Ok(Response::new()
        .add_attribute("action", "add_whitelist")
        .add_attribute("address", addr.to_string()))
}

fn exec_remove_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    if WHITELIST.may_load(deps.storage, &addr)?.is_none() {
        return Err(ContractError::NotInWhitelist { address });
    }
    WHITELIST.remove(deps.storage, &addr);
    Ok(Response::new()
        .add_attribute("action", "remove_whitelist")
        .add_attribute("address", addr.to_string()))
}

fn exec_set_whitelist_enabled(
    deps: DepsMut,
    info: MessageInfo,
    enabled: bool,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.whitelist_enabled = enabled;
        Ok(cfg)
    })?;
    Ok(Response::new()
        .add_attribute("action", "set_whitelist_enabled")
        .add_attribute("enabled", enabled.to_string()))
}

fn exec_freeze(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    reason: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    FROZEN.save(deps.storage, &addr, &reason)?;
    Ok(Response::new()
        .add_attribute("action", "freeze_account")
        .add_attribute("address", addr.to_string())
        .add_attribute("reason", reason))
}

fn exec_unfreeze(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    FROZEN.remove(deps.storage, &addr);
    Ok(Response::new()
        .add_attribute("action", "unfreeze_account")
        .add_attribute("address", addr.to_string()))
}

fn exec_set_aggregator(
    deps: DepsMut,
    info: MessageInfo,
    aggregator_addr: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&aggregator_addr)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.aggregator_addr = Some(addr.clone());
        Ok(cfg)
    })?;
    Ok(Response::new()
        .add_attribute("action", "set_aggregator")
        .add_attribute("aggregator", addr.to_string()))
}

fn exec_set_por_enforced(
    deps: DepsMut,
    info: MessageInfo,
    enforced: bool,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.por_enforced = enforced;
        Ok(cfg)
    })?;
    Ok(Response::new()
        .add_attribute("action", "set_por_enforced")
        .add_attribute("enforced", enforced.to_string()))
}
fn exec_update_reserve_uri(
    deps: DepsMut,
    info: MessageInfo,
    uri: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.reserve_uri = uri.clone();
        Ok(cfg)
    })?;
    Ok(Response::new()
        .add_attribute("action", "update_reserve_uri")
        .add_attribute("uri", uri))
}

fn exec_transfer_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let new_addr = deps.api.addr_validate(&new_admin)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.admin = new_addr.clone();
        Ok(cfg)
    })?;
    Ok(Response::new()
        .add_attribute("action", "transfer_admin")
        .add_attribute("new_admin", new_addr.to_string()))
}

fn exec_pause(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.paused = true;
        Ok(cfg)
    })?;
    Ok(Response::new().add_attribute("action", "pause"))
}

fn exec_unpause(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.paused = false;
        Ok(cfg)
    })?;
    Ok(Response::new().add_attribute("action", "unpause"))
}

// ── Query ─────────────────────────────────────────────────────────────────────

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // CW20 standard queries
        QueryMsg::Balance { address } =>
            to_json_binary(&cw20::query_balance(deps, address)?),
        QueryMsg::TokenInfo {} =>
            to_json_binary(&cw20::query_token_info(deps)?),
        QueryMsg::Allowance { owner, spender } =>
            to_json_binary(&cw20_base::allowances::query_allowance(deps, owner, spender)?),
        QueryMsg::AllAllowances { owner, start_after, limit } =>
            to_json_binary(&cw20_base::enumerable::query_owner_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllAccounts { start_after, limit } =>
            to_json_binary(&cw20_base::enumerable::query_all_accounts(deps, start_after, limit)?),

        // CLSSC specific
        QueryMsg::Config {} => query_config(deps),
        QueryMsg::IsMinter { address } => query_is_minter(deps, address),
        QueryMsg::ListMinters { start_after, limit } => query_list_minters(deps, start_after, limit),
        QueryMsg::IsWhitelisted { address } => query_is_whitelisted(deps, address),
        QueryMsg::IsFrozen { address } => query_is_frozen(deps, address),
        QueryMsg::ReserveStats {} => query_reserve_stats(deps),
    }
}

fn query_config(deps: Deps) -> StdResult<Binary> {
    let cfg = CONFIG.load(deps.storage)?;
    let token_info = TOKEN_INFO.load(deps.storage)?;
    to_json_binary(&ConfigResponse {
        admin: cfg.admin.to_string(),
        total_minted: Uint128::from(cfg.total_minted),
        total_burned: Uint128::from(cfg.total_burned),
        circulating_supply: token_info.total_supply,
        reserve_uri: cfg.reserve_uri,
        whitelist_enabled: cfg.whitelist_enabled,
        paused: cfg.paused,
    })
}

fn query_is_minter(deps: Deps, address: String) -> StdResult<Binary> {
    let addr = deps.api.addr_validate(&address)?;
    let label = MINTERS.may_load(deps.storage, &addr)?;
    let cap = MINT_CAPS.may_load(deps.storage, &addr)?.map(Uint128::from);
    let lifetime = LIFETIME_MINTED.may_load(deps.storage, &addr)?.unwrap_or(0u128);
    to_json_binary(&IsMinterResponse {
        is_minter: label.is_some(),
        label,
        cap,
        lifetime_minted: Uint128::from(lifetime),
    })
}

fn query_list_minters(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let limit = limit.unwrap_or(30) as usize;
    let start = start_after.map(|s| deps.api.addr_validate(&s)).transpose()?;

    let minters: Vec<MinterInfo> = MINTERS
        .range(deps.storage, None, None, Order::Ascending)
        .filter(|item| {
            if let (Ok((addr, _)), Some(ref start_addr)) = (item, &start) {
                addr > start_addr
            } else {
                true
            }
        })
        .take(limit)
        .filter_map(|item| item.ok())
        .map(|(addr, label)| {
            let cap = MINT_CAPS.may_load(deps.storage, &addr).ok().flatten().map(Uint128::from);
            let lifetime = LIFETIME_MINTED.may_load(deps.storage, &addr).ok().flatten().unwrap_or(0u128);
            MinterInfo {
                address: addr.to_string(),
                label,
                cap,
                lifetime_minted: Uint128::from(lifetime),
            }
        })
        .collect();

    to_json_binary(&ListMintersResponse { minters })
}

fn query_is_whitelisted(deps: Deps, address: String) -> StdResult<Binary> {
    let addr = deps.api.addr_validate(&address)?;
    let ok = WHITELIST.may_load(deps.storage, &addr)?.unwrap_or(false);
    to_json_binary(&ok)
}

fn query_is_frozen(deps: Deps, address: String) -> StdResult<Binary> {
    let addr = deps.api.addr_validate(&address)?;
    let reason = FROZEN.may_load(deps.storage, &addr)?;
    to_json_binary(&reason)
}

fn query_reserve_stats(deps: Deps) -> StdResult<Binary> {
    let cfg = CONFIG.load(deps.storage)?;
    let token_info = TOKEN_INFO.load(deps.storage)?;
    to_json_binary(&ReserveStatsResponse {
        total_minted: Uint128::from(cfg.total_minted),
        total_burned: Uint128::from(cfg.total_burned),
        circulating_supply: token_info.total_supply,
    })
}

// ── Migrate ───────────────────────────────────────────────────────────────────

pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
}
