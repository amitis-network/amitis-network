use cosmwasm_std::{
    to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Response, StdResult, Uint128, WasmQuery,
};
use cw2::set_contract_version;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;
use crate::msg::{
    AcceptedDenom, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
    QuoteResponse, ReserveResponse, StateResponse,
};
use crate::state::{Config, State, CONFIG, STATE};

const CONTRACT_NAME: &str    = "amitis:amitis-swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Fee constants — immutable at deployment
const FEE_BPS: u128      = 30;   // 0.30% total fee
const REBATE_BPS: u128   = 24;   // 0.24% rebated to user (80% of fee)
const NET_FEE_BPS: u128  = 6;    // 0.06% net (20% of fee)
const LP_SPLIT: u128     = 80;   // 80% of net → DEX pool / community pool
const VAL_SPLIT: u128    = 20;   // 20% of net → validator fee collector
const BPS_BASE: u128     = 10_000;

// VPT base price — 1 VPT = $7.50 = 750 cents (immutable)
const VPT_BASE_PRICE_CENTS: u64 = 750;

// ── Pool oracle response types ────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct SpotPriceResponse {
    pub amts_per_vpt: String, // uamts per uVPT in micro units
    pub vpt_per_amts: String,
    pub vpt_reserve:  String,
    pub amts_reserve: String,
}

#[derive(Serialize, Deserialize)]
struct SpotPriceData {
    pub data: SpotPriceResponse,
}

// ── INSTANTIATE ───────────────────────────────────────────────────────────────

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        amts_denom:              msg.amts_denom.clone(),
        accepted_denoms:         msg.accepted_denoms,
        floor_price_cents:       msg.floor_price_cents,
        ceiling_price_cents:     msg.ceiling_price_cents,
        community_pool_address:  msg.community_pool_address,
        dex_contract_address:    msg.dex_contract_address,
        fee_collector_address:   msg.fee_collector_address,
        oracle_address:          msg.oracle_address,
        // Governance address must be set to x/gov module address
        // amitis10d07y265gmmuvt4z0w9aw880jnsr700jkzsk3e
        governance_address:      msg.governance_address,
    };

    let state = State {
        paused:                false,
        total_swaps:           0,
        total_amts_dispensed:  Uint128::zero(),
        total_usdc_received:   Uint128::zero(),
    };

    CONFIG.save(deps.storage, &config)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("amts_denom", msg.amts_denom)
        .add_attribute("floor_price_cents", msg.floor_price_cents.to_string())
        .add_attribute("ceiling_price_cents", msg.ceiling_price_cents.to_string())
        .add_attribute("governance", config.governance_address))
}

// ── ORACLE ────────────────────────────────────────────────────────────────────

/// Query VPT/AMTS pool for current spot price.
/// Returns AMTS price in cents.
/// Falls back to floor_price_cents if oracle query fails.
fn get_current_rate_cents(deps: Deps, config: &Config) -> u64 {
    let oracle_addr = match &config.oracle_address {
        Some(a) => a.clone(),
        None => return config.floor_price_cents,
    };

    #[derive(serde::Serialize)]
    struct SpotPriceQuery { spot_price: SpotPriceEmpty }
    #[derive(serde::Serialize)]
    struct SpotPriceEmpty {}

    let query_msg = cosmwasm_std::to_json_binary(&SpotPriceQuery {
        spot_price: SpotPriceEmpty {}
    }).unwrap_or_default();

    let req = QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: oracle_addr,
        msg: query_msg,
    });

    let result: Result<SpotPriceData, _> = deps.querier.query(&req);

    match result {
        Ok(resp) => {
            // amts_per_vpt is uamts per uVPT
            // Since 1 VPT = $7.50:
            //   price_cents = VPT_BASE_PRICE_CENTS * 1_000_000 / amts_per_vpt
            // If amts_per_vpt = 1_000_000 → price = 750 cents = $7.50
            // If amts_per_vpt = 500_000  → price = 1500 cents = $15.00 (AMTS doubled)
            // If amts_per_vpt = 2_000_000 → price = 375 cents = $3.75 (AMTS halved)
            let amts_per_vpt: u128 = resp.data.amts_per_vpt.parse().unwrap_or(1_000_000);
            if amts_per_vpt == 0 {
                return config.floor_price_cents;
            }
            let price = (VPT_BASE_PRICE_CENTS as u128)
                .saturating_mul(1_000_000)
                / amts_per_vpt;
            let price_cents = price as u64;
            // Clamp to floor — never go below floor
            price_cents.max(config.floor_price_cents)
        }
        Err(_) => {
            // Oracle unavailable — use floor price (safe default)
            config.floor_price_cents
        }
    }
}

// ── EXECUTE ───────────────────────────────────────────────────────────────────

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Swap { min_amts_out } =>
            execute_swap(deps, env, info, min_amts_out),
        ExecuteMsg::Pause {} =>
            execute_governance(deps, info, |s| { s.paused = true; }, "pause"),
        ExecuteMsg::Resume {} =>
            execute_governance(deps, info, |s| { s.paused = false; }, "resume"),
        ExecuteMsg::UpdateCeiling { new_ceiling_cents } =>
            execute_update_ceiling(deps, info, new_ceiling_cents),
        ExecuteMsg::UpdateAcceptedDenoms { denoms } =>
            execute_update_denoms(deps, info, denoms),
        ExecuteMsg::UpdateFeeRouting {
            community_pool_address,
            fee_collector_address,
            dex_contract_address,
        } => execute_update_fee_routing(
            deps, info,
            community_pool_address,
            fee_collector_address,
            dex_contract_address,
        ),
        ExecuteMsg::UpdateOracle { oracle_address } =>
            execute_update_oracle(deps, info, oracle_address),
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    min_amts_out: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    if state.paused {
        return Err(ContractError::Paused {});
    }
    if info.funds.is_empty() {
        return Err(ContractError::NoFunds {});
    }
    if info.funds.len() > 1 {
        return Err(ContractError::MultipleFunds {});
    }

    let coin = &info.funds[0];
    if coin.amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    // Find accepted denom
    let denom_cfg = config
        .accepted_denoms
        .iter()
        .find(|d| d.denom == coin.denom)
        .ok_or_else(|| ContractError::UnsupportedDenom { denom: coin.denom.clone() })?;

    // Get current rate from oracle (falls back to floor if unavailable)
    let rate_cents = get_current_rate_cents(deps.as_ref(), &config);

    // Validate price bounds — auto-pause if out of range
    if rate_cents > config.ceiling_price_cents {
        return Err(ContractError::PriceOutOfBounds {
            price:   rate_cents,
            floor:   config.floor_price_cents,
            ceiling: config.ceiling_price_cents,
        });
    }

    let input = coin.amount.u128();
    let upc   = denom_cfg.usd_micro_cents_per_base as u128;

    // Calculate AMTS out
    // Formula: uamts_out = (input_base_units × upc × 1_000_000) / (rate_cents × 1_000_000_000_000)
    // For USDC (upc=100): uamts_out = input_uusdc × 100 / rate_cents
    // Example: 7_500_000 uusdc at 750 cents → 7_500_000 × 100 / 750 = 1_000_000 uamts = 1 AMTS ✓
    let numerator = input
        .checked_mul(upc)
        .and_then(|n| n.checked_mul(1_000_000))
        .ok_or(ContractError::Overflow {})?;
    let denominator = (rate_cents as u128)
        .checked_mul(1_000_000_000_000)
        .ok_or(ContractError::Overflow {})?;

    if denominator == 0 {
        return Err(ContractError::Overflow {});
    }
    let gross_amts = numerator / denominator;
    if gross_amts == 0 {
        return Err(ContractError::ZeroAmount {});
    }

    // Fee calculations (on input token)
    let total_fee    = input * FEE_BPS / BPS_BASE;
    let rebate_amt   = input * REBATE_BPS / BPS_BASE;
    let net_fee      = input * NET_FEE_BPS / BPS_BASE;
    let lp_amt       = net_fee * LP_SPLIT / 100;
    let val_amt      = net_fee * VAL_SPLIT / 100;

    // AMTS out is based on gross (fee is in input token, not AMTS)
    let amts_out = Uint128::from(gross_amts);

    // Slippage check
    if amts_out < min_amts_out {
        return Err(ContractError::SlippageExceeded {
            min: min_amts_out.to_string(),
            got: amts_out.to_string(),
        });
    }

    // Check reserve
    let reserve = deps.querier.query_balance(&env.contract.address, &config.amts_denom)?;
    if reserve.amount < amts_out {
        return Err(ContractError::InsufficientReserve {
            need: amts_out.to_string(),
            have: reserve.amount.to_string(),
        });
    }

    // Build messages
    let mut msgs: Vec<CosmosMsg> = vec![];

    // 1. AMTS to user
    msgs.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin { denom: config.amts_denom.clone(), amount: amts_out }],
    }));

    // 2. Fee rebate back to user (80% of fee in input token)
    if rebate_amt > 0 {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom:  coin.denom.clone(),
                amount: Uint128::from(rebate_amt),
            }],
        }));
    }

    // 3. LP portion → DEX pool (or community pool if no DEX configured)
    let lp_dest = config.dex_contract_address
        .as_deref()
        .unwrap_or(&config.community_pool_address);
    if lp_amt > 0 {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: lp_dest.to_string(),
            amount: vec![Coin {
                denom:  coin.denom.clone(),
                amount: Uint128::from(lp_amt),
            }],
        }));
    }

    // 4. Validator fee
    if val_amt > 0 {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.fee_collector_address.clone(),
            amount: vec![Coin {
                denom:  coin.denom.clone(),
                amount: Uint128::from(val_amt),
            }],
        }));
    }

    // Update state
    state.total_swaps          += 1;
    state.total_amts_dispensed += amts_out;
    state.total_usdc_received  += coin.amount;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action",        "swap")
        .add_attribute("sender",        info.sender.to_string())
        .add_attribute("input_denom",   &coin.denom)
        .add_attribute("input_amount",  coin.amount.to_string())
        .add_attribute("amts_out",      amts_out.to_string())
        .add_attribute("rebate",        rebate_amt.to_string())
        .add_attribute("lp_routed",     lp_amt.to_string())
        .add_attribute("validator_fee", val_amt.to_string())
        .add_attribute("rate_cents",    rate_cents.to_string()))
}

// ── GOVERNANCE HELPERS ────────────────────────────────────────────────────────

fn assert_governance(deps: Deps, info: &MessageInfo) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender.to_string() != config.governance_address {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

fn execute_governance<F: FnOnce(&mut State)>(
    deps: DepsMut,
    info: MessageInfo,
    mutate: F,
    action: &str,
) -> Result<Response, ContractError> {
    assert_governance(deps.as_ref(), &info)?;
    let mut state = STATE.load(deps.storage)?;
    mutate(&mut state);
    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("action", action))
}

fn execute_update_ceiling(
    deps: DepsMut,
    info: MessageInfo,
    new_ceiling_cents: u64,
) -> Result<Response, ContractError> {
    assert_governance(deps.as_ref(), &info)?;
    let mut config = CONFIG.load(deps.storage)?;
    config.ceiling_price_cents = new_ceiling_cents;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "update_ceiling")
        .add_attribute("new_ceiling_cents", new_ceiling_cents.to_string()))
}

fn execute_update_denoms(
    deps: DepsMut,
    info: MessageInfo,
    denoms: Vec<AcceptedDenom>,
) -> Result<Response, ContractError> {
    assert_governance(deps.as_ref(), &info)?;
    let mut config = CONFIG.load(deps.storage)?;
    config.accepted_denoms = denoms;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_accepted_denoms"))
}

fn execute_update_fee_routing(
    deps: DepsMut,
    info: MessageInfo,
    community_pool_address: Option<String>,
    fee_collector_address:  Option<String>,
    dex_contract_address:   Option<String>,
) -> Result<Response, ContractError> {
    assert_governance(deps.as_ref(), &info)?;
    let mut config = CONFIG.load(deps.storage)?;
    if let Some(a) = community_pool_address { config.community_pool_address = a; }
    if let Some(a) = fee_collector_address  { config.fee_collector_address  = a; }
    config.dex_contract_address = dex_contract_address;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_fee_routing"))
}

fn execute_update_oracle(
    deps: DepsMut,
    info: MessageInfo,
    oracle_address: Option<String>,
) -> Result<Response, ContractError> {
    assert_governance(deps.as_ref(), &info)?;
    let mut config = CONFIG.load(deps.storage)?;
    config.oracle_address = oracle_address;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_oracle"))
}

// ── QUERY ─────────────────────────────────────────────────────────────────────

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {}              => to_json_binary(&query_config(deps)?),
        QueryMsg::Quote { denom, amount} => to_json_binary(&query_quote(deps, denom, amount)?),
        QueryMsg::Reserve {}             => to_json_binary(&query_reserve(deps, env)?),
        QueryMsg::State {}               => to_json_binary(&query_state(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let c = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        amts_denom:             c.amts_denom,
        accepted_denoms:        c.accepted_denoms,
        floor_price_cents:      c.floor_price_cents,
        ceiling_price_cents:    c.ceiling_price_cents,
        community_pool_address: c.community_pool_address,
        dex_contract_address:   c.dex_contract_address,
        fee_collector_address:  c.fee_collector_address,
        oracle_address:         c.oracle_address,
        governance_address:     c.governance_address,
    })
}

fn query_quote(deps: Deps, denom: String, amount: Uint128) -> StdResult<QuoteResponse> {
    let config = CONFIG.load(deps.storage)?;

    let denom_cfg = config
        .accepted_denoms
        .iter()
        .find(|d| d.denom == denom)
        .ok_or_else(|| cosmwasm_std::StdError::generic_err(
            format!("unsupported denom: {}", denom)
        ))?;

    let rate_cents = get_current_rate_cents(deps, &config);
    let input      = amount.u128();
    let upc        = denom_cfg.usd_micro_cents_per_base as u128;

    let num        = input.saturating_mul(upc).saturating_mul(1_000_000);
    let denom_calc = (rate_cents as u128).saturating_mul(1_000_000_000_000);
    let amts_out   = if denom_calc > 0 { num / denom_calc } else { 0 };

    let fee_amount        = Uint128::from(input * FEE_BPS / BPS_BASE);
    let rebate_amount     = Uint128::from(input * REBATE_BPS / BPS_BASE);
    let net_fee           = Uint128::from(input * NET_FEE_BPS / BPS_BASE);
    let community_pool_amt= Uint128::from(input * NET_FEE_BPS / BPS_BASE * LP_SPLIT / 100);

    Ok(QuoteResponse {
        input_denom:           denom,
        input_amount:          amount,
        amts_out:              Uint128::from(amts_out),
        rate_cents,
        fee_amount,
        rebate_amount,
        net_fee,
        community_pool_amount: community_pool_amt,
    })
}

fn query_reserve(deps: Deps, env: Env) -> StdResult<ReserveResponse> {
    let config  = CONFIG.load(deps.storage)?;
    let balance = deps.querier.query_balance(&env.contract.address, &config.amts_denom)?;
    Ok(ReserveResponse { amts_balance: balance.amount })
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let config = CONFIG.load(deps.storage)?;
    let state  = STATE.load(deps.storage)?;
    Ok(StateResponse {
        paused:                state.paused,
        current_rate_cents:    get_current_rate_cents(deps, &config),
        floor_price_cents:     config.floor_price_cents,
        ceiling_price_cents:   config.ceiling_price_cents,
        total_swaps:           state.total_swaps,
        total_amts_dispensed:  state.total_amts_dispensed,
        total_usdc_received:   state.total_usdc_received,
    })
}
