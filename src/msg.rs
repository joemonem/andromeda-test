use cosmwasm_std::Coin;
use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::NFT;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Sell {
        token_id: String,
        contract_address: String,
        price: Coin,
        expiration: Expiration,
    },
    Buy {
        token_id: String,
    },
    Delist {
        token_id: String,
        contract_address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    GetListing { token_id: String },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListingResponse {
    pub nft: NFT,
}
