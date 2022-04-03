use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Map;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub count: i32,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct NFT {
    pub token_id: String,
    pub owner: String,
    pub contract_address: String,
    pub price: Coin,
    pub expiration: Expiration,
}

pub const LIST: Map<String, NFT> = Map::new("List");
