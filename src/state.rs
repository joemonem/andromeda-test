use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::{Item, Map};

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
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct AuctionNft {
    pub token_id: String,
    pub owner: String,
    pub contract_address: String,
    pub starting_price: Coin,
    pub expiration: Expiration,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Bidder {
    pub address: String,
    pub bid: Coin,
}

pub const AUCTION_LIST: Map<String, AuctionNft> = Map::new("AuctionList");
pub const LIST: Map<String, NFT> = Map::new("List");
pub const HIGHEST_BIDDER: Map<String, Bidder> = Map::new("Bidders");
