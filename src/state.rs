use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub token: Addr,
    pub owner: Addr
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("token_distributor");

pub const WITHDRAWABLE: Map<Addr, Uint128> = Map::new("withdrawable");

pub const FEE_COLLECTED: Item<Uint128> = Item::new("fee_collected");
