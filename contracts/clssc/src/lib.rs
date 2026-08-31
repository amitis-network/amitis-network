// CLSSC — Closed-Loop Stablecoin System
// Deployed on Amitis Network
//
// Architecture:
//   - CW20 base token (name/symbol/decimals/balances/allowances)
//   - Controlled mint: only authorized minters can mint (Meridian payroll processor)
//   - Controlled burn: any holder can burn their own tokens (redemption to fiat)
//   - Admin: can add/remove minters, freeze accounts, update reserve URI
//   - Proof of Reserve: on-chain URI pointing to Chainlink PoR feed
//   - Peg: 1 CLSSC = 1 USD (enforced off-chain by reserve management)
//   - Closed-loop: transfer whitelist optional — can be enabled to restrict
//     transfers to verified Meridian employees only

pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, MigrateMsg};

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        contract::instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        contract::execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        contract::query(deps, env, msg)
    }

    #[entry_point]
    pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
        contract::migrate(deps, env, msg)
    }
}
