use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Order, Response, StdResult, SubMsg, Uint128, WasmMsg, Reply,
    SubMsgResponse,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, FeeInfoResponse, InstantiateMsg, MigrateMsg, QueryMsg,
    RegistryStatsResponse,
};
use crate::state::{
    ApprovedAsset, Config, DeploymentRecord, IssuerRecord, IssuerStatus,
    ReserveType, APPROVED_ASSETS, CONFIG, CONTRACT_TO_DEPLOYMENT,
    DEPLOYMENTS, ISSUERS, ISSUER_DEPLOYMENTS, NEXT_DEPLOYMENT_ID,
};

const CONTRACT_NAME: &str = "meridian-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Reply ID for stablecoin instantiation
const REPLY_INSTANTIATE: u64 = 1;

// ── Instantiate ───────────────────────────────────────────────────────────────

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if msg.mint_fee_bps > 10000 { return Err(ContractError::InvalidBps { bps: msg.mint_fee_bps }); }
    if msg.burn_fee_bps > 10000 { return Err(ContractError::InvalidBps { bps: msg.burn_fee_bps }); }

    let admin = deps.api.addr_validate(&msg.admin)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    let fee_collector = deps.api.addr_validate(&msg.fee_collector)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;

    CONFIG.save(deps.storage, &Config {
        admin: admin.clone(),
        clssc_code_id: msg.clssc_code_id,
        mint_fee_bps: msg.mint_fee_bps,
        burn_fee_bps: msg.burn_fee_bps,
        fee_collector: fee_collector.clone(),
        total_fees_collected: Uint128::zero(),
        total_deployments: 0,
        accepting_registrations: true,
    })?;

    NEXT_DEPLOYMENT_ID.save(deps.storage, &1u64)?;

    // Seed approved assets
    let default_assets = vec![
        ("USD_FIAT", "US Dollar Fiat", true),
        ("T_BILLS", "US Treasury Bills", true),
        ("USDC", "USD Coin (USDC)", true),
        ("USDT", "Tether (USDT)", true),
        ("EUR_FIAT", "Euro Fiat", true),
        ("GBP_FIAT", "British Pound Fiat", true),
    ];
    for (id, name, approved) in default_assets {
        APPROVED_ASSETS.save(deps.storage, id, &ApprovedAsset {
            id: id.to_string(),
            name: name.to_string(),
            approved,
        })?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate_meridian_factory")
        .add_attribute("admin", admin.to_string())
        .add_attribute("clssc_code_id", msg.clssc_code_id.to_string())
        .add_attribute("mint_fee_bps", msg.mint_fee_bps.to_string())
        .add_attribute("burn_fee_bps", msg.burn_fee_bps.to_string()))
}

// ── Execute ───────────────────────────────────────────────────────────────────

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RegisterIssuer { legal_name, jurisdiction, kyc_ref, reserve_type, reserve_uri } =>
            exec_register_issuer(deps, env, info, legal_name, jurisdiction, kyc_ref, reserve_type, reserve_uri),

        ExecuteMsg::ApproveIssuer { address } =>
            exec_approve_issuer(deps, env, info, address),

        ExecuteMsg::SuspendIssuer { address, reason } =>
            exec_suspend_issuer(deps, info, address, reason),

        ExecuteMsg::RevokeIssuer { address, reason } =>
            exec_revoke_issuer(deps, info, address, reason),

        ExecuteMsg::ReinstateIssuer { address } =>
            exec_reinstate_issuer(deps, info, address),

        ExecuteMsg::DeployStablecoin { name, symbol, decimals, peg_currency, reserve_type, reserve_uri, whitelist_enabled, initial_whitelist } =>
            exec_deploy_stablecoin(deps, env, info, name, symbol, decimals, peg_currency, reserve_type, reserve_uri, whitelist_enabled, initial_whitelist),

        ExecuteMsg::ReportMint { issuer, recipient, amount, memo } =>
            exec_report_mint(deps, info, issuer, recipient, amount, memo),

        ExecuteMsg::ReportBurn { issuer, burner, amount, memo } =>
            exec_report_burn(deps, info, issuer, burner, amount, memo),

        ExecuteMsg::PauseContract { contract_addr } =>
            exec_pause_contract(deps, info, contract_addr),

        ExecuteMsg::UnpauseContract { contract_addr } =>
            exec_unpause_contract(deps, info, contract_addr),

        ExecuteMsg::UpdateConfig { mint_fee_bps, burn_fee_bps, fee_collector, accepting_registrations, clssc_code_id } =>
            exec_update_config(deps, info, mint_fee_bps, burn_fee_bps, fee_collector, accepting_registrations, clssc_code_id),

        ExecuteMsg::SetApprovedAsset { id, name, approved } =>
            exec_set_approved_asset(deps, info, id, name, approved),

        ExecuteMsg::TransferAdmin { new_admin } =>
            exec_transfer_admin(deps, info, new_admin),
    }
}

// ── Issuer registration ───────────────────────────────────────────────────────

fn exec_register_issuer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    legal_name: String,
    jurisdiction: String,
    kyc_ref: String,
    reserve_type: ReserveType,
    reserve_uri: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if !config.accepting_registrations {
        return Err(ContractError::RegistrationsClosed {});
    }
    if ISSUERS.may_load(deps.storage, &info.sender)?.is_some() {
        return Err(ContractError::IssuerAlreadyRegistered { address: info.sender.to_string() });
    }

    ISSUERS.save(deps.storage, &info.sender, &IssuerRecord {
        address: info.sender.clone(),
        legal_name: legal_name.clone(),
        jurisdiction: jurisdiction.clone(),
        kyc_ref: kyc_ref.clone(),
        status: IssuerStatus::Pending,
        reserve_type,
        reserve_uri,
        registered_at: env.block.height,
        approved_at: None,
        deployments: 0,
        lifetime_minted: Uint128::zero(),
        lifetime_burned: Uint128::zero(),
        lifetime_fees: Uint128::zero(),
    })?;

    Ok(Response::new()
        .add_attribute("action", "register_issuer")
        .add_attribute("issuer", info.sender.to_string())
        .add_attribute("legal_name", legal_name)
        .add_attribute("jurisdiction", jurisdiction)
        .add_attribute("kyc_ref", kyc_ref)
        .add_attribute("status", "pending"))
}

fn exec_approve_issuer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    let mut issuer = ISSUERS.load(deps.storage, &addr)
        .map_err(|_| ContractError::IssuerNotFound { address: address.clone() })?;

    issuer.status = IssuerStatus::Approved;
    issuer.approved_at = Some(env.block.height);
    ISSUERS.save(deps.storage, &addr, &issuer)?;

    Ok(Response::new()
        .add_attribute("action", "approve_issuer")
        .add_attribute("issuer", address))
}

fn exec_suspend_issuer(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    reason: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    let mut issuer = ISSUERS.load(deps.storage, &addr)
        .map_err(|_| ContractError::IssuerNotFound { address: address.clone() })?;
    issuer.status = IssuerStatus::Suspended;
    ISSUERS.save(deps.storage, &addr, &issuer)?;
    Ok(Response::new()
        .add_attribute("action", "suspend_issuer")
        .add_attribute("issuer", address)
        .add_attribute("reason", reason))
}

fn exec_revoke_issuer(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    reason: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    let mut issuer = ISSUERS.load(deps.storage, &addr)
        .map_err(|_| ContractError::IssuerNotFound { address: address.clone() })?;
    issuer.status = IssuerStatus::Revoked;
    ISSUERS.save(deps.storage, &addr, &issuer)?;
    Ok(Response::new()
        .add_attribute("action", "revoke_issuer")
        .add_attribute("issuer", address)
        .add_attribute("reason", reason))
}

fn exec_reinstate_issuer(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&address)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    let mut issuer = ISSUERS.load(deps.storage, &addr)
        .map_err(|_| ContractError::IssuerNotFound { address: address.clone() })?;
    issuer.status = IssuerStatus::Approved;
    ISSUERS.save(deps.storage, &addr, &issuer)?;
    Ok(Response::new()
        .add_attribute("action", "reinstate_issuer")
        .add_attribute("issuer", address))
}

// ── Deploy stablecoin ─────────────────────────────────────────────────────────

fn exec_deploy_stablecoin(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    symbol: String,
    decimals: u8,
    peg_currency: String,
    reserve_type: ReserveType,
    reserve_uri: String,
    whitelist_enabled: bool,
    initial_whitelist: Vec<String>,
) -> Result<Response, ContractError> {
    // Must be approved issuer
    let issuer = ISSUERS.load(deps.storage, &info.sender)
        .map_err(|_| ContractError::IssuerNotFound { address: info.sender.to_string() })?;

    if !matches!(issuer.status, IssuerStatus::Approved) {
        return Err(ContractError::IssuerNotApproved {
            address: info.sender.to_string(),
            status: format!("{:?}", issuer.status),
        });
    }

    if reserve_uri.is_empty() { return Err(ContractError::MissingReserveUri {}); }

    let config = CONFIG.load(deps.storage)?;
    let deployment_id = NEXT_DEPLOYMENT_ID.load(deps.storage)?;

    // Build CLSSC instantiate message
    // Factory address is the admin so it can enforce rules
    let init_msg = serde_json::json!({
        "name": name,
        "symbol": symbol,
        "decimals": decimals,
        "admin": env.contract.address.to_string(),
        "initial_minters": [[info.sender.to_string(), format!("issuer-{}", info.sender)]],
        "reserve_uri": reserve_uri,
        "whitelist_enabled": whitelist_enabled,
        "initial_whitelist": initial_whitelist,
        "initial_balances": [],
        "mint_caps": []
    });

    let instantiate_msg = WasmMsg::Instantiate {
        admin: Some(env.contract.address.to_string()),
        code_id: config.clssc_code_id,
        msg: to_json_binary(&init_msg)?,
        funds: vec![],
        label: format!("meridian-{}-{}", symbol.to_lowercase(), deployment_id),
    };

    // Store pending deployment record (address filled in on reply)
    DEPLOYMENTS.save(deps.storage, deployment_id, &DeploymentRecord {
        id: deployment_id,
        issuer: info.sender.clone(),
        contract_addr: env.contract.address.clone(), // placeholder — updated on reply
        name: name.clone(),
        symbol: symbol.clone(),
        decimals,
        peg_currency: peg_currency.clone(),
        reserve_type,
        reserve_uri: reserve_uri.clone(),
        deployed_at: env.block.height,
        active: false, // activated on reply
        circulating_supply: Uint128::zero(),
        lifetime_minted: Uint128::zero(),
        lifetime_burned: Uint128::zero(),
        lifetime_fees: Uint128::zero(),
    })?;

    NEXT_DEPLOYMENT_ID.save(deps.storage, &(deployment_id + 1))?;
    ISSUER_DEPLOYMENTS.save(deps.storage, (&info.sender, deployment_id), &true)?;

    // Update issuer deployment count
    let mut updated_issuer = issuer;
    updated_issuer.deployments += 1;
    ISSUERS.save(deps.storage, &info.sender, &updated_issuer)?;

    // Update factory deployment count
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.total_deployments += 1;
        Ok(cfg)
    })?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(instantiate_msg),
            REPLY_INSTANTIATE,
        ))
        .add_attribute("action", "deploy_stablecoin")
        .add_attribute("issuer", info.sender.to_string())
        .add_attribute("deployment_id", deployment_id.to_string())
        .add_attribute("name", name)
        .add_attribute("symbol", symbol)
        .add_attribute("peg_currency", peg_currency))
}

// ── Reply handler — captures contract address after instantiation ─────────────

pub fn reply(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    if msg.id != REPLY_INSTANTIATE {
        return Ok(Response::new());
    }

    let response = msg.result.into_result()
        .map_err(|e| ContractError::InstantiationFailed { msg: e })?;

    // Extract contract address from events
    let contract_addr = extract_contract_addr(&response)?;
    let addr = deps.api.addr_validate(&contract_addr)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;

    // Find the most recent deployment (the one we just created)
    let next_id = NEXT_DEPLOYMENT_ID.load(deps.storage)?;
    let deployment_id = next_id - 1;

    let mut deployment = DEPLOYMENTS.load(deps.storage, deployment_id)?;
    deployment.contract_addr = addr.clone();
    deployment.active = true;
    DEPLOYMENTS.save(deps.storage, deployment_id, &deployment)?;

    CONTRACT_TO_DEPLOYMENT.save(deps.storage, &addr, &deployment_id)?;

    Ok(Response::new()
        .add_attribute("action", "stablecoin_deployed")
        .add_attribute("deployment_id", deployment_id.to_string())
        .add_attribute("contract_addr", addr.to_string()))
}

fn extract_contract_addr(response: &SubMsgResponse) -> Result<String, ContractError> {
    for event in &response.events {
        if event.ty == "instantiate" {
            for attr in &event.attributes {
                if attr.key == "_contract_address" || attr.key == "contract_address" {
                    return Ok(attr.value.clone());
                }
            }
        }
    }
    Err(ContractError::InstantiationFailed {
        msg: "Could not find contract address in reply events".to_string(),
    })
}

// ── Fee reporting ─────────────────────────────────────────────────────────────

fn exec_report_mint(
    deps: DepsMut,
    info: MessageInfo,
    issuer: String,
    _recipient: String,
    amount: Uint128,
    _memo: Option<String>,
) -> Result<Response, ContractError> {
    if amount.is_zero() { return Err(ContractError::ZeroAmount {}); }

    // Verify caller is a registered contract
    let _deployment_id = CONTRACT_TO_DEPLOYMENT
        .may_load(deps.storage, &info.sender)?
        .ok_or(ContractError::UnauthorizedCallback {})?;

    let config = CONFIG.load(deps.storage)?;
    let fee = amount.multiply_ratio(config.mint_fee_bps, 10000u128);

    // Update deployment stats
    let deployment_id = CONTRACT_TO_DEPLOYMENT.load(deps.storage, &info.sender)?;
    let mut deployment = DEPLOYMENTS.load(deps.storage, deployment_id)?;
    deployment.circulating_supply += amount;
    deployment.lifetime_minted += amount;
    deployment.lifetime_fees += fee;
    DEPLOYMENTS.save(deps.storage, deployment_id, &deployment)?;

    // Update issuer stats
    let issuer_addr = deps.api.addr_validate(&issuer)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    if let Ok(mut issuer_rec) = ISSUERS.load(deps.storage, &issuer_addr) {
        issuer_rec.lifetime_minted += amount;
        issuer_rec.lifetime_fees += fee;
        ISSUERS.save(deps.storage, &issuer_addr, &issuer_rec)?;
    }

    // Update global fee counter
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.total_fees_collected += fee;
        Ok(cfg)
    })?;

    Ok(Response::new()
        .add_attribute("action", "report_mint")
        .add_attribute("contract", info.sender.to_string())
        .add_attribute("amount", amount.to_string())
        .add_attribute("fee", fee.to_string()))
}

fn exec_report_burn(
    deps: DepsMut,
    info: MessageInfo,
    issuer: String,
    _burner: String,
    amount: Uint128,
    _memo: Option<String>,
) -> Result<Response, ContractError> {
    if amount.is_zero() { return Err(ContractError::ZeroAmount {}); }

    let _deployment_id = CONTRACT_TO_DEPLOYMENT
        .may_load(deps.storage, &info.sender)?
        .ok_or(ContractError::UnauthorizedCallback {})?;

    let config = CONFIG.load(deps.storage)?;
    let fee = amount.multiply_ratio(config.burn_fee_bps, 10000u128);

    let deployment_id = CONTRACT_TO_DEPLOYMENT.load(deps.storage, &info.sender)?;
    let mut deployment = DEPLOYMENTS.load(deps.storage, deployment_id)?;
    deployment.circulating_supply = deployment.circulating_supply.saturating_sub(amount);
    deployment.lifetime_burned += amount;
    deployment.lifetime_fees += fee;
    DEPLOYMENTS.save(deps.storage, deployment_id, &deployment)?;

    let issuer_addr = deps.api.addr_validate(&issuer)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    if let Ok(mut issuer_rec) = ISSUERS.load(deps.storage, &issuer_addr) {
        issuer_rec.lifetime_burned += amount;
        issuer_rec.lifetime_fees += fee;
        ISSUERS.save(deps.storage, &issuer_addr, &issuer_rec)?;
    }

    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.total_fees_collected += fee;
        Ok(cfg)
    })?;

    Ok(Response::new()
        .add_attribute("action", "report_burn")
        .add_attribute("contract", info.sender.to_string())
        .add_attribute("amount", amount.to_string())
        .add_attribute("fee", fee.to_string()))
}

// ── Contract pause/unpause ────────────────────────────────────────────────────

fn exec_pause_contract(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&contract_addr)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;

    // Verify it's a registered contract
    let dep_id = CONTRACT_TO_DEPLOYMENT.may_load(deps.storage, &addr)?
        .ok_or(ContractError::ContractNotFound { addr: contract_addr.clone() })?;
    let mut dep = DEPLOYMENTS.load(deps.storage, dep_id)?;
    dep.active = false;
    DEPLOYMENTS.save(deps.storage, dep_id, &dep)?;

    // Send pause message to the contract
    let pause_msg = WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_json_binary(&serde_json::json!({"pause": {}}))?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(pause_msg))
        .add_attribute("action", "pause_contract")
        .add_attribute("contract", contract_addr))
}

fn exec_unpause_contract(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&contract_addr)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;

    let dep_id = CONTRACT_TO_DEPLOYMENT.may_load(deps.storage, &addr)?
        .ok_or(ContractError::ContractNotFound { addr: contract_addr.clone() })?;
    let mut dep = DEPLOYMENTS.load(deps.storage, dep_id)?;
    dep.active = true;
    DEPLOYMENTS.save(deps.storage, dep_id, &dep)?;

    let unpause_msg = WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_json_binary(&serde_json::json!({"unpause": {}}))?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(unpause_msg))
        .add_attribute("action", "unpause_contract")
        .add_attribute("contract", contract_addr))
}

// ── Config / admin ────────────────────────────────────────────────────────────

fn exec_update_config(
    deps: DepsMut,
    info: MessageInfo,
    mint_fee_bps: Option<u64>,
    burn_fee_bps: Option<u64>,
    fee_collector: Option<String>,
    accepting_registrations: Option<bool>,
    clssc_code_id: Option<u64>,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        if let Some(v) = mint_fee_bps { cfg.mint_fee_bps = v; }
        if let Some(v) = burn_fee_bps { cfg.burn_fee_bps = v; }
        if let Some(v) = fee_collector {
            cfg.fee_collector = deps.api.addr_validate(&v)?;
        }
        if let Some(v) = accepting_registrations { cfg.accepting_registrations = v; }
        if let Some(v) = clssc_code_id { cfg.clssc_code_id = v; }
        Ok(cfg)
    })?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

fn exec_set_approved_asset(
    deps: DepsMut,
    info: MessageInfo,
    id: String,
    name: String,
    approved: bool,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    APPROVED_ASSETS.save(deps.storage, &id, &ApprovedAsset { id: id.clone(), name, approved })?;
    Ok(Response::new()
        .add_attribute("action", "set_approved_asset")
        .add_attribute("id", id)
        .add_attribute("approved", approved.to_string()))
}

fn exec_transfer_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    only_admin(&deps.as_ref(), &info.sender)?;
    let addr = deps.api.addr_validate(&new_admin)
        .map_err(|e| ContractError::InvalidAddress { msg: e.to_string() })?;
    CONFIG.update(deps.storage, |mut cfg| -> StdResult<_> {
        cfg.admin = addr.clone();
        Ok(cfg)
    })?;
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

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),

        QueryMsg::Issuer { address } => {
            let addr = deps.api.addr_validate(&address)?;
            to_json_binary(&ISSUERS.load(deps.storage, &addr)?)
        },

        QueryMsg::ListIssuers { start_after, limit, .. } => {
            let limit = limit.unwrap_or(30) as usize;
            let start = start_after.map(|s| deps.api.addr_validate(&s)).transpose()?;
            let issuers: Vec<IssuerRecord> = ISSUERS
                .range(deps.storage, None, None, Order::Ascending)
                .filter(|item| {
                    if let (Ok((addr, _)), Some(ref s)) = (item, &start) {
                        addr > s
                    } else { true }
                })
                .take(limit)
                .filter_map(|r| r.ok())
                .map(|(_, v)| v)
                .collect();
            to_json_binary(&issuers)
        },

        QueryMsg::Deployment { id } =>
            to_json_binary(&DEPLOYMENTS.load(deps.storage, id)?),

        QueryMsg::DeploymentByContract { contract_addr } => {
            let addr = deps.api.addr_validate(&contract_addr)?;
            let id = CONTRACT_TO_DEPLOYMENT.load(deps.storage, &addr)?;
            to_json_binary(&DEPLOYMENTS.load(deps.storage, id)?)
        },

        QueryMsg::ListDeployments { start_after, limit, .. } => {
            let limit = limit.unwrap_or(30) as usize;
            let start = start_after.unwrap_or(0);
            let deps_list: Vec<DeploymentRecord> = DEPLOYMENTS
                .range(deps.storage, None, None, Order::Ascending)
                .filter(|item| {
                    if let Ok((id, _)) = item { *id > start } else { true }
                })
                .take(limit)
                .filter_map(|r| r.ok())
                .map(|(_, v)| v)
                .collect();
            to_json_binary(&deps_list)
        },

        QueryMsg::ApprovedAssets {} => {
            let assets: Vec<ApprovedAsset> = APPROVED_ASSETS
                .range(deps.storage, None, None, Order::Ascending)
                .filter_map(|r| r.ok())
                .map(|(_, v)| v)
                .collect();
            to_json_binary(&assets)
        },

        QueryMsg::RegistryStats {} => {
            let config = CONFIG.load(deps.storage)?;
            let all_issuers: Vec<IssuerRecord> = ISSUERS
                .range(deps.storage, None, None, Order::Ascending)
                .filter_map(|r| r.ok())
                .map(|(_, v)| v)
                .collect();
            let approved = all_issuers.iter()
                .filter(|i| matches!(i.status, IssuerStatus::Approved))
                .count() as u64;
            to_json_binary(&RegistryStatsResponse {
                total_issuers: all_issuers.len() as u64,
                approved_issuers: approved,
                total_deployments: config.total_deployments,
                total_fees_collected: config.total_fees_collected,
            })
        },

        QueryMsg::FeeInfo {} => {
            let config = CONFIG.load(deps.storage)?;
            to_json_binary(&FeeInfoResponse {
                mint_fee_bps: config.mint_fee_bps,
                burn_fee_bps: config.burn_fee_bps,
                fee_collector: config.fee_collector.to_string(),
                total_fees_collected: config.total_fees_collected,
            })
        },
    }
}

// ── Migrate ───────────────────────────────────────────────────────────────────

pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
}
