use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    from_binary, to_binary, CosmosMsg, WasmMsg, SubMsg, Uint128,
};

use crate::contract::{instantiate, execute, query};
use crate::msg::{InstantiateMsg, ExecuteMsg, QueryMsg, Cw20HookMsg, DepositMsg, WithdrawableMsg, WithdrawMsg, WithdrawAllMsg, WithdrawFeeMsg};
use crate::error::{ContractError};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::test::mock_querier::mock_dependencies;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        token: "asset0001".to_string(),
        owner: "addr0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let owner: String = from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Owner()).unwrap()).unwrap();
    assert_eq!("addr0000".to_string(), owner);
}

#[test]
fn execute_deposit() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        token: "asset0001".to_string(),
        owner: "addr0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_token_balances(&[(
        &"asset0001".to_string(),
        &[(&"addr0001".to_string(), &Uint128::from(1000000u128))],
    )]);

    let deposit_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        msg: to_binary(&Cw20HookMsg::Deposit(DepositMsg{
            addr1: "addr0002".to_string(),
            addr2: "addr0003".to_string(),
        })).unwrap(),
        amount: Uint128::from(100u128),
    });

    let deposit_info = mock_info("asset0001", &[]);

    execute(deps.as_mut(), mock_env(), deposit_info, deposit_msg).unwrap();

    let withdrawable1: Uint128 = from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Withdrawable(WithdrawableMsg{ addr: "addr0002".to_string() })).unwrap()).unwrap();
    assert_eq!(Uint128::from(47u128), withdrawable1);
    let withdrawable2: Uint128 = from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Withdrawable(WithdrawableMsg{ addr: "addr0003".to_string() })).unwrap()).unwrap();
    assert_eq!(Uint128::from(48u128), withdrawable2);
}

#[test]
fn execute_withdraw() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        token: "asset0001".to_string(),
        owner: "addr0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_token_balances(&[(
        &"asset0001".to_string(),
        &[
            (&"addr0001".to_string(), &Uint128::from(1000000u128)),
        ],
    )]);

    let deposit_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        msg: to_binary(&Cw20HookMsg::Deposit(DepositMsg{
            addr1: "addr0002".to_string(),
            addr2: "addr0003".to_string(),
        })).unwrap(),
        amount: Uint128::from(1000u128),
    });

    let deposit_info = mock_info("asset0001", &[]);

    execute(deps.as_mut(), mock_env(), deposit_info, deposit_msg).unwrap();

    let withdraw_msg = ExecuteMsg::Withdraw(WithdrawMsg{ amount: Uint128::from(300u128) });

    let withdraw_info = mock_info("addr0002", &[]);

    let res = execute(deps.as_mut(), mock_env(), withdraw_info, withdraw_msg).unwrap();

    let withdrawable: Uint128 = from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Withdrawable(WithdrawableMsg{ addr: "addr0002".to_string() })).unwrap()).unwrap();
    assert_eq!(Uint128::from(175u128), withdrawable);

    let msg_transfer = res.messages.get(0).expect("no message");
    assert_eq!(
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0001".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0002".to_string(),
                amount: Uint128::from(300u128),
            })
            .unwrap(),
            funds: vec![],
        })),
        msg_transfer,
    );
}

#[test]
fn execute_withdraw_all() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        token: "asset0001".to_string(),
        owner: "addr0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_token_balances(&[(
        &"asset0001".to_string(),
        &[
            (&"addr0001".to_string(), &Uint128::from(1000000u128)),
        ],
    )]);

    let deposit_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        msg: to_binary(&Cw20HookMsg::Deposit(DepositMsg{
            addr1: "addr0002".to_string(),
            addr2: "addr0003".to_string(),
        })).unwrap(),
        amount: Uint128::from(1000u128),
    });

    let deposit_info = mock_info("asset0001", &[]);

    execute(deps.as_mut(), mock_env(), deposit_info, deposit_msg).unwrap();

    let withdraw_msg = ExecuteMsg::WithdrawAll(WithdrawAllMsg{});

    let withdraw_info = mock_info("addr0002", &[]);

    let res = execute(deps.as_mut(), mock_env(), withdraw_info, withdraw_msg).unwrap();

    let withdrawable: Uint128 = from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Withdrawable(WithdrawableMsg{ addr: "addr0002".to_string() })).unwrap()).unwrap();
    assert_eq!(Uint128::zero(), withdrawable);

    let msg_transfer = res.messages.get(0).expect("no message");
    assert_eq!(
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0001".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0002".to_string(),
                amount: Uint128::from(475u128),
            })
            .unwrap(),
            funds: vec![],
        })),
        msg_transfer,
    );
}

#[test]
fn execute_withdraw_fee() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        token: "asset0001".to_string(),
        owner: "addr0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_token_balances(&[(
        &"asset0001".to_string(),
        &[
            (&"addr0001".to_string(), &Uint128::from(1000000u128)),
        ],
    )]);

    let deposit_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        msg: to_binary(&Cw20HookMsg::Deposit(DepositMsg{
            addr1: "addr0002".to_string(),
            addr2: "addr0003".to_string(),
        })).unwrap(),
        amount: Uint128::from(1000u128),
    });

    let deposit_info = mock_info("asset0001", &[]);

    execute(deps.as_mut(), mock_env(), deposit_info, deposit_msg).unwrap();

    let withdraw_fee_msg = ExecuteMsg::WithdrawFee(WithdrawFeeMsg{});

    let withdraw_fee_info = mock_info("addr0001", &[]);

    let res = execute(deps.as_mut(), mock_env(), withdraw_fee_info, withdraw_fee_msg.clone()).unwrap_err();
    match res {
        ContractError::Unauthorized {} => (),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let withdraw_fee_info = mock_info("addr0000", &[]);

    let res = execute(deps.as_mut(), mock_env(), withdraw_fee_info, withdraw_fee_msg).unwrap();

    let msg_transfer = res.messages.get(0).expect("no message");
    assert_eq!(
        &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0001".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(50u128),
            })
            .unwrap(),
            funds: vec![],
        })),
        msg_transfer,
    );
}
