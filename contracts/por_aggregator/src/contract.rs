use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo,
    Order, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, ListOraclesResponse, MigrateMsg,
    MintCheckResponse, OracleInfo, QueryMsg,
};
use crate::state::{
    Config, OracleSubmission, ReserveStatus, RoundData,
    CONFIG, LATEST_ROUND, ORACLES, PENDING_SUBMISSIONS,
    ROUND_HISTORY, ROUND_ID,
};

const CONTRACT_NAME: &str = "meridian-por-aggregator";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_STALENESS_SECS: u64 = 86400; // 24 hours
const DEFAULT_MIN_SUBMISSIONS: u64 = 1;
const MAX_HISTORY: u64 = 100;

// ── Instantiate ───────────────────────────────────────────────────────────────

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;

    let factory = msg.factory.as_deref()
        .map(|f| deps.api.addr_validate(f))
        .transpose()
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;

    CONFIG.save(deps.storage, &Config {
        admin: admin.clone(),
        factory,
        staleness_threshold_secs: msg.staleness_threshold_secs.unwrap_or(DEFAULT_STALENESS_SECS),
        min_submissions: msg.min_submissions.unwrap_or(DEFAULT_MIN_SUBMISSIONS),
        paused: false,
        description: msg.description.clone(),
        denomination: msg.denomination.clone(),
    })?;

    ROUND_ID.save(deps.storage, &0u64)?;

    for (addr_str, label) in &msg.initial_oracles {
        let addr = deps.api.addr_validate(addr_str)
            .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
        ORACLES.save(deps.storage, &addr, label)?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate_por_aggregator")
        .add_attribute("admin", admin.to_string())
        .add_attribute("description", msg.description)
        .add_attribute("denomination", msg.denomination)
        .add_attribute("staleness_threshold_secs",
            msg.staleness_threshold_secs.unwrap_or(DEFAULT_STALENESS_SECS).to_string()))
}

// ── Execute ───────────────────────────────────────────────────────────────────

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SubmitReserve { amount, reference } =>
            exec_submit_reserve(deps, env, info, amount, reference),

        ExecuteMsg::AddOracle { address, label } =>
            exec_add_oracle(deps, info, address, label),

        ExecuteMsg::RemoveOracle { address } =>
            exec_remove_oracle(deps, info, address),

        ExecuteMsg::UpdateConfig { staleness_threshold_secs, min_submissions, factory } =>
            exec_update_config(deps, info, staleness_threshold_secs, min_submissions, factory),

        ExecuteMsg::AdminOverride { amount, reason } =>
            exec_admin_override(deps, env, info, amount, reason),

        ExecuteMsg::Pause {} => exec_pause(deps, info),
        ExecuteMsg::Unpause {} => exec_unpause(deps, info),
        ExecuteMsg::TransferAdmin { new_admin } => exec_transfer_admin(deps, info, new_admin),
    }
}

// ── Submit reserve (oracle relayer) ──────────────────────────────────────────

fn exec_submit_reserve(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    reference: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.paused { return Err(ContractError::Paused {}); }

    // Must be an authorized oracle
    if ORACLES.may_load(deps.storage, &info.sender)?.is_none() {
        return Err(ContractError::NotOracle { address: info.sender.to_string() });
    }

    let now = env.block.time.seconds();

    // Check for duplicate submission in current round
    if PENDING_SUBMISSIONS.may_load(deps.storage, &info.sender)?.is_some() {
        return Err(ContractError::AlreadySubmitted { address: info.sender.to_string() });
    }

    // Record this submission
    PENDING_SUBMISSIONS.save(deps.storage, &info.sender, &OracleSubmission {
        oracle: info.sender.clone(),
        value: amount,
        timestamp: now,
    })?;

    // Count submissions
    let submissions: Vec<OracleSubmission> = PENDING_SUBMISSIONS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .map(|(_, v)| v)
        .collect();

    let submission_count = submissions.len() as u64;
    let mut resp = Response::new()
        .add_attribute("action", "submit_reserve")
        .add_attribute("oracle", info.sender.to_string())
        .add_attribute("amount", amount.to_string())
        .add_attribute("submissions", submission_count.to_string());

    if let Some(r) = &reference {
        resp = resp.add_attribute("reference", r);
    }

    // Finalize round if we have enough submissions
    if submission_count >= config.min_submissions {
        // Compute median of submitted values
        let mut values: Vec<u128> = submissions.iter().map(|s| s.value.u128()).collect();
        values.sort();
        let median = values[values.len() / 2];
        let finalized_amount = Uint128::from(median);

        // Advance round ID
        let round_id = ROUND_ID.load(deps.storage)? + 1;
        ROUND_ID.save(deps.storage, &round_id)?;

        let round = RoundData {
            round_id,
            reserve_amount: finalized_amount,
            started_at: now,
            updated_at: now,
            block_height: env.block.height,
            submission_count,
            status: ReserveStatus::Active,
        };

        LATEST_ROUND.save(deps.storage, &round)?;

        // Archive to history (keep last MAX_HISTORY rounds)
        ROUND_HISTORY.save(deps.storage, round_id, &round)?;
        if round_id > MAX_HISTORY {
            ROUND_HISTORY.remove(deps.storage, round_id - MAX_HISTORY);
        }

        // Clear pending submissions for next round
        let keys: Vec<_> = PENDING_SUBMISSIONS
            .keys(deps.storage, None, None, Order::Ascending)
            .filter_map(|k| k.ok())
            .collect();
        for k in keys {
            PENDING_SUBMISSIONS.remove(deps.storage, &k);
        }

        resp = resp
            .add_attribute("round_finalized", round_id.to_string())
            .add_attribute("finalized_reserve", finalized_amount.to_string());
    }

    Ok(resp)
}

// ── Admin override ────────────────────────────────────────────────────────────

fn exec_admin_override(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    reason: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;

    let now = env.block.time.seconds();
    let round_id = ROUND_ID.load(deps.storage)? + 1;
    ROUND_ID.save(deps.storage, &round_id)?;

    let round = RoundData {
        round_id,
        reserve_amount: amount,
        started_at: now,
        updated_at: now,
        block_height: env.block.height,
        submission_count: 0,
        status: ReserveStatus::Active,
    };

    LATEST_ROUND.save(deps.storage, &round)?;
    ROUND_HISTORY.save(deps.storage, round_id, &round)?;

    Ok(Response::new()
        .add_attribute("action", "admin_override")
        .add_attribute("WARNING", "reserve_manually_set_by_admin")
        .add_attribute("amount", amount.to_string())
        .add_attribute("reason", reason)
        .add_attribute("round_id", round_id.to_string()))
}

// ── Oracle management ─────────────────────────────────────────────────────────

fn exec_add_oracle(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    label: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    if ORACLES.may_load(deps.storage, &addr)?.is_some() {
        return Err(ContractError::OracleAlreadyExists { address });
    }
    ORACLES.save(deps.storage, &addr, &label)?;
    Ok(Response::new()
        .add_attribute("action", "add_oracle")
        .add_attribute("address", addr.to_string())
        .add_attribute("label", label))
}

fn exec_remove_oracle(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    if ORACLES.may_load(deps.storage, &addr)?.is_none() {
        return Err(ContractError::NotOracle { address });
    }
    ORACLES.remove(deps.storage, &addr);
    Ok(Response::new()
        .add_attribute("action", "remove_oracle")
        .add_attribute("address", addr.to_string()))
}

fn exec_update_config(
    deps: DepsMut,
    info: MessageInfo,
    staleness_threshold_secs: Option<u64>,
    min_submissions: Option<u64>,
    factory: Option<String>,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        if let Some(v) = staleness_threshold_secs { cfg.staleness_threshold_secs = v; }
        if let Some(v) = min_submissions { cfg.min_submissions = v; }
        if let Some(f) = factory {
            cfg.factory = Some(deps.api.addr_validate(&f)?);
        }
        Ok(cfg)
    })?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

fn exec_pause(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> { cfg.paused = true; Ok(cfg) })?;
    Ok(Response::new().add_attribute("action", "pause_aggregator"))
}

fn exec_unpause(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> { cfg.paused = false; Ok(cfg) })?;
    Ok(Response::new().add_attribute("action", "unpause_aggregator"))
}

fn exec_transfer_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&new_admin)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> { cfg.admin = addr.clone(); Ok(cfg) })?;
    Ok(Response::new()
        .add_attribute("action", "transfer_admin")
        .add_attribute("new_admin", addr.to_string()))
}

fn only_admin(deps: &Deps, sender: &cosmwasm_std::Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if *sender != config.admin { return Err(ContractError::Unauthorized {}); }
    Ok(())
}

// ── Query ─────────────────────────────────────────────────────────────────────

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::LatestRound {} => {
            let round = LATEST_ROUND.may_load(deps.storage)?;
            to_json_binary(&round)
        },

        QueryMsg::Round { round_id } => {
            let round = ROUND_HISTORY.may_load(deps.storage, round_id)?;
            to_json_binary(&round)
        },

        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),

        QueryMsg::MintCheck { mint_amount, circulating_supply } => {
            to_json_binary(&query_mint_check(deps, env, mint_amount, circulating_supply)?)
        },

        QueryMsg::ListOracles {} => {
            let oracles: Vec<OracleInfo> = ORACLES
                .range(deps.storage, None, None, Order::Ascending)
                .filter_map(|r| r.ok())
                .map(|(addr, label)| OracleInfo { address: addr.to_string(), label })
                .collect();
            to_json_binary(&ListOraclesResponse { oracles })
        },

        QueryMsg::RoundHistory { limit } => {
            let limit = limit.unwrap_or(20) as usize;
            let rounds: Vec<RoundData> = ROUND_HISTORY
                .range(deps.storage, None, None, Order::Descending)
                .take(limit)
                .filter_map(|r| r.ok())
                .map(|(_, v)| v)
                .collect();
            to_json_binary(&rounds)
        },
    }
}

fn query_mint_check(
    deps: Deps,
    env: Env,
    mint_amount: Uint128,
    circulating_supply: Uint128,
) -> StdResult<MintCheckResponse> {
    let config = CONFIG.load(deps.storage)?;
    let now = env.block.time.seconds();

    // Paused check
    if config.paused {
        return Ok(MintCheckResponse {
            valid: false,
            reserve: Uint128::zero(),
            circulating_supply,
            mint_amount,
            headroom: Uint128::zero(),
            reason: Some("Aggregator is paused".to_string()),
            data_age_secs: 0,
        });
    }

    // Uninitialized check
    let round = match LATEST_ROUND.may_load(deps.storage)? {
        Some(r) => r,
        None => return Ok(MintCheckResponse {
            valid: false,
            reserve: Uint128::zero(),
            circulating_supply,
            mint_amount,
            headroom: Uint128::zero(),
            reason: Some("Reserve feed not initialized — no oracle submissions yet".to_string()),
            data_age_secs: 0,
        }),
    };

    let data_age_secs = now.saturating_sub(round.updated_at);

    // Staleness check
    if data_age_secs > config.staleness_threshold_secs {
        return Ok(MintCheckResponse {
            valid: false,
            reserve: round.reserve_amount,
            circulating_supply,
            mint_amount,
            headroom: Uint128::zero(),
            reason: Some(format!(
                "Reserve data is stale — {}s old, threshold {}s",
                data_age_secs, config.staleness_threshold_secs
            )),
            data_age_secs,
        });
    }

    // Reserve sufficiency check
    let proposed_total = circulating_supply.checked_add(mint_amount)
        .unwrap_or(Uint128::MAX);

    if proposed_total > round.reserve_amount {
        let headroom = round.reserve_amount.saturating_sub(circulating_supply);
        return Ok(MintCheckResponse {
            valid: false,
            reserve: round.reserve_amount,
            circulating_supply,
            mint_amount,
            headroom,
            reason: Some(format!(
                "Mint would exceed reserves: {} circulating + {} requested > {} attested",
                circulating_supply, mint_amount, round.reserve_amount
            )),
            data_age_secs,
        });
    }

    let headroom = round.reserve_amount
        .saturating_sub(circulating_supply)
        .saturating_sub(mint_amount);

    Ok(MintCheckResponse {
        valid: true,
        reserve: round.reserve_amount,
        circulating_supply,
        mint_amount,
        headroom,
        reason: None,
        data_age_secs,
    })
}

// ── Migrate ───────────────────────────────────────────────────────────────────

pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
}
