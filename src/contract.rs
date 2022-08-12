#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, WasmMsg, Uint128,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{Cw20HookMsg, DepositMsg, ExecuteMsg, InstantiateMsg, QueryMsg, WithdrawMsg, WithdrawAllMsg, WithdrawableMsg};
use crate::state::{ContractInfo, CONTRACT_INFO, WITHDRAWABLE};

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
        ExecuteMsg::Withdraw(msg) => withdraw(deps, info, msg),
        ExecuteMsg::WithdrawAll(msg) => withdraw_all(deps, info, msg),
        ExecuteMsg::Receive(msg) => deposit(deps, info, msg),
    }
}

fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    msg: WithdrawMsg,
) -> Result<Response, ContractError> {
    return _withdraw(deps, info, msg.amount);
}

fn withdraw_all(
    deps: DepsMut,
    info: MessageInfo,
    _msg: WithdrawAllMsg,
) -> Result<Response, ContractError> {
    let amount = WITHDRAWABLE.load(deps.storage, info.sender.clone())?;

    return _withdraw(deps, info, amount);
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

    let withdrawable = WITHDRAWABLE.load(deps.storage, info.sender.clone())?;
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
        Ok(Cw20HookMsg::Deposit(msg)) => {
            let DepositMsg {
                addr1,
                addr2,
            } = msg;
            // Validations
            if token_contract != contract_info.token {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Invalid token".to_string(),
                }));
            }

            // Handle the real "deposit".
            let amount1 = amount / Uint128::from(2u128);
            let amount2 = amount - amount1;

            let withdrawable1 = WITHDRAWABLE.load(deps.storage, deps.api.addr_validate(&addr1)?)?;
            let withdrawable2 = WITHDRAWABLE.load(deps.storage, deps.api.addr_validate(&addr2)?)?;

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
        QueryMsg::Owner() => to_binary(&get_owner(deps)?),
        QueryMsg::Withdrawable(msg) => to_binary(&withdrawable(deps, msg)?),
    }
}

fn get_owner(deps: Deps) -> StdResult<String> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    Ok(contract_info.owner.to_string())
}

fn withdrawable(deps: Deps, msg: WithdrawableMsg) -> StdResult<Uint128> {
    let amount = WITHDRAWABLE.load(deps.storage, deps.api.addr_validate(&msg.addr)?)?;

    Ok(amount)
}
