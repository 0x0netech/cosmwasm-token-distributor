#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, WasmMsg, Uint128,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{ContractInfo, CONTRACT_INFO, WITHDRAWABLE, FEE_COLLECTED};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract_info = ContractInfo {
        token: deps.api.addr_validate(&msg.token)?,
        owner: deps.api.addr_validate(&msg.owner)?,
    };
    CONTRACT_INFO.save(deps.storage, &contract_info)?;
    FEE_COLLECTED.save(deps.storage, &Uint128::zero())?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Withdraw { amount } => _withdraw(deps, info, amount),
        ExecuteMsg::WithdrawAll {} => withdraw_all(deps, info),
        ExecuteMsg::WithdrawFee {} => withdraw_fee(deps, info),
        ExecuteMsg::Receive(msg) => deposit(deps, info, msg),
    }
}

fn withdraw_all(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let amount = match WITHDRAWABLE.may_load(deps.storage, info.sender.clone())? {
        Some(val) => val,
        None => Uint128::zero()
    };

    return _withdraw(deps, info, amount);
}

fn withdraw_fee(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let token = contract_info.token;

    // validate owner
    if contract_info.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let fee = FEE_COLLECTED.load(deps.storage)?;
    FEE_COLLECTED.save(deps.storage, &Uint128::zero())?;

    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount: fee,
        })?,
        funds: vec![],
    })];

    Ok(Response::default().add_messages(msgs))
}

fn _withdraw(
    deps: DepsMut,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let token = contract_info.token;

    if amount.is_zero() {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid zero amount".to_string(),
        }));
    }

    let withdrawable = match WITHDRAWABLE.may_load(deps.storage, info.sender.clone())? {
        Some(val) => val,
        None => Uint128::zero()
    };
    if amount > withdrawable {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Insufficient amount".to_string(),
        }));
    }

    WITHDRAWABLE.save(deps.storage, info.sender.clone(), &(withdrawable - amount))?;

    // Handle the real "withdraw"
    let recipient = deps.api.addr_validate(info.sender.as_str())?;
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount: amount,
        })?,
        funds: vec![],
    })];

    Ok(Response::default().add_messages(msgs))
}

fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let token_contract = info.sender;
    let amount = cw20_msg.amount;

    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    // Deserialize the message for the params
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Deposit { addr1, addr2 }) => {
            // Validations
            if token_contract != contract_info.token {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Invalid token".to_string(),
                }));
            }

            let fee = Uint128::from(amount.u128() * 50u128 / 1000u128);
            let total_fee = FEE_COLLECTED.load(deps.storage)? + fee;
            FEE_COLLECTED.save(deps.storage, &total_fee)?;
            let send_amount = amount - fee;

            // Handle the real "deposit".
            let amount1 = send_amount / Uint128::from(2u128);
            let amount2 = send_amount - amount1;

            let withdrawable1 = match WITHDRAWABLE.may_load(deps.storage, deps.api.addr_validate(&addr1)?)? {
                Some(val) => val,
                None => Uint128::zero()
            };
            let withdrawable2 = match WITHDRAWABLE.may_load(deps.storage, deps.api.addr_validate(&addr2)?)? {
                Some(val) => val,
                None => Uint128::zero()
            };

            WITHDRAWABLE.save(deps.storage, deps.api.addr_validate(&addr1)?, &(withdrawable1 + amount1))?;
            WITHDRAWABLE.save(deps.storage, deps.api.addr_validate(&addr2)?, &(withdrawable2 + amount2))?;

            Ok(Response::default())
        }
        Err(_) => Err(ContractError::Unauthorized {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_binary(&get_owner(deps)?),
        QueryMsg::Withdrawable { addr } => to_binary(&withdrawable(deps, addr)?),
    }
}

fn get_owner(deps: Deps) -> StdResult<String> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    Ok(contract_info.owner.to_string())
}

fn withdrawable(deps: Deps, addr: String) -> StdResult<Uint128> {
    match WITHDRAWABLE.may_load(deps.storage, deps.api.addr_validate(&addr)?)? {
        Some(val) => Ok(val),
        None => Ok(Uint128::zero())
    }
}
