use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractResult, Empty, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::panic;

use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier,
    token_querier: TokenQuerier,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<String, HashMap<String, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&String, &[(&String, &Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(addr.to_string(), **balance);
        }

        balances_map.insert(contract_addr.to_string(), contract_balances_map);
    }
    balances_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => match from_binary(msg).unwrap() {
                Cw20QueryMsg::TokenInfo {} => {
                    let balances: &HashMap<String, Uint128> =
                        match self.token_querier.balances.get(contract_addr) {
                            Some(balances) => balances,
                            None => {
                                return SystemResult::Err(SystemError::InvalidRequest {
                                    error: format!(
                                        "No balance info exists for the contract {}",
                                        contract_addr
                                    ),
                                    request: msg.as_slice().into(),
                                })
                            }
                        };

                    let mut total_supply = Uint128::zero();

                    for balance in balances {
                        total_supply += *balance.1;
                    }

                    SystemResult::Ok(ContractResult::Ok(
                        to_binary(&TokenInfoResponse {
                            name: "mAAPL".to_string(),
                            symbol: "mAAPL".to_string(),
                            decimals: 8,
                            total_supply,
                        })
                        .unwrap(),
                    ))
                }
                Cw20QueryMsg::Balance { address } => {
                    let balances: &HashMap<String, Uint128> =
                        match self.token_querier.balances.get(contract_addr) {
                            Some(balances) => balances,
                            None => {
                                return SystemResult::Err(SystemError::InvalidRequest {
                                    error: format!(
                                        "No balance info exists for the contract {}",
                                        contract_addr
                                    ),
                                    request: msg.as_slice().into(),
                                })
                            }
                        };

                    let balance = match balances.get(&address) {
                        Some(v) => *v,
                        None => {
                            return SystemResult::Ok(ContractResult::Ok(
                                to_binary(&Cw20BalanceResponse {
                                    balance: Uint128::zero(),
                                })
                                .unwrap(),
                            ));
                        }
                    };

                    SystemResult::Ok(ContractResult::Ok(
                        to_binary(&Cw20BalanceResponse { balance }).unwrap(),
                    ))
                }

                _ => panic!("DO NOT ENTER HERE"),
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
        }
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }
}

#[cfg(test)]
mod mock_exception {
    use cosmwasm_std::Binary;

    use super::*;

    #[test]
    fn raw_query_err() {
        let deps = mock_dependencies(&[]);
        assert_eq!(
            deps.querier.raw_query(&[]),
            SystemResult::Err(SystemError::InvalidRequest {
                error: "Parsing query request: Error parsing into type cosmwasm_std::query::QueryRequest<cosmwasm_std::results::empty::Empty>: EOF while parsing a JSON value.".to_string(),
                request: Binary(vec![])
            })
        );
    }

    #[test]
    fn none_tokens_info_will_err() {
        let deps = mock_dependencies(&[]);

        let msg = to_binary(&Cw20QueryMsg::TokenInfo {}).unwrap();

        assert_eq!(
            deps.querier
                .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: "token0000".to_string(),
                    msg: msg.clone()
                })),
            SystemResult::Err(SystemError::InvalidRequest {
                error: "No balance info exists for the contract token0000".to_string(),
                request: msg
            })
        )
    }

    #[test]
    fn none_tokens_balance_will_err() {
        let deps = mock_dependencies(&[]);

        let msg = to_binary(&Cw20QueryMsg::Balance {
            address: "address0000".to_string(),
        })
        .unwrap();

        assert_eq!(
            deps.querier
                .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: "token0000".to_string(),
                    msg: msg.clone()
                })),
            SystemResult::Err(SystemError::InvalidRequest {
                error: "No balance info exists for the contract token0000".to_string(),
                request: msg
            })
        )
    }

    #[test]
    #[should_panic]
    fn none_tokens_minter_will_panic() {
        let deps = mock_dependencies(&[]);

        let msg = to_binary(&Cw20QueryMsg::Minter {}).unwrap();

        deps.querier
            .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "token0000".to_string(),
                msg,
            }));
    }
}
