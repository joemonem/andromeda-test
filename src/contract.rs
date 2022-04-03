use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, Expiration, OwnerOfResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, ListingResponse, QueryMsg};
use crate::state::{LIST, NFT};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:{{marketplace}}";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn get_token_owner(
    deps: Deps,
    token_id: String,
    contract_address: String,
) -> Result<String, ContractError> {
    let res: OwnerOfResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_address,
        msg: to_binary(&Cw721QueryMsg::OwnerOf {
            token_id,
            include_expired: None,
        })?,
    }))?;
    Ok(res.owner)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Sell {
            token_id,
            contract_address,
            price,
            expiration,
        } => execute_sell(
            deps,
            env,
            info,
            token_id,
            contract_address,
            price,
            expiration,
        ),
        ExecuteMsg::Buy { token_id } => execute_buy(deps, env, info, token_id),
        ExecuteMsg::Delist {
            token_id,
            contract_address,
        } => execute_delist(deps, env, info, token_id, contract_address),
    }
}

pub fn execute_sell(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    contract_address: String,
    price: Coin,
    expiration: Expiration,
) -> Result<Response, ContractError> {
    // check if NFT is already listed
    if LIST.has(deps.storage, token_id.clone()) == true {
        return Err(ContractError::AlreadyListed {});
    }
    // retrieve NFT owner
    let owner = get_token_owner(deps.as_ref(), token_id.clone(), contract_address.clone())?;
    // check if sender is owner
    if info.sender.to_string() != owner {
        return Err(ContractError::Unauthorized {});
    }
    // valid price
    if price.amount <= Uint128::from(0 as u32) {
        return Err(ContractError::InvalidAmount {});
    }
    // valid denomination
    if price.denom != "uusd".to_string() {
        return Err(ContractError::InvalidDenomination {});
    }
    // // valid expiration
    // if expiration.is_expired(&env.block) == true {
    //     return Err(ContractError::Expired {});
    // }
    // transfer permission to the marketplace smart contract by sending a message to the NFT smart contract
    // Implement the NFT's components
    let nft = NFT {
        token_id: token_id.clone(),
        owner,
        contract_address,
        price: price.clone(),
        expiration,
    };
    // add the NFT to the list of NFTs for sale

    LIST.save(deps.storage, token_id.clone(), &nft)?;
    // send response
    let res = Response::new()
        .add_attribute("action", "list")
        .add_attribute("ID", token_id)
        .add_attribute("expires", expiration.to_string())
        .add_attribute("price", price.to_string().clone());

    Ok(res)
}
pub fn execute_delist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    contract_address: String,
) -> Result<Response, ContractError> {
    // check if NFT is already listed
    if LIST.has(deps.storage, token_id.clone()) == false {
        return Err(ContractError::NotListed {});
    }
    // retrieve NFT owner
    let owner = get_token_owner(deps.as_ref(), token_id.clone(), contract_address)?;
    // check if sender is owner
    if info.sender.to_string() != owner {
        return Err(ContractError::Unauthorized {});
    }
    // remove from list
    LIST.remove(deps.storage, token_id.clone());
    Ok(Response::new().add_attribute("Delisted", token_id.clone()))
}

pub fn execute_buy(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    // check if NFT is listed
    if LIST.has(deps.storage, token_id.clone()) == false {
        return Err(ContractError::NotListed {});
    }
    // load the list of NFTs
    let nft = LIST.load(deps.storage, token_id.clone())?;
    // check price
    if info.funds[0].amount != nft.price.amount {
        return Err(ContractError::InvalidAmount {});
    }
    // check denom
    if info.funds[0].denom != "UST".to_string() {
        return Err(ContractError::InvalidDenomination {});
    }
    // // check expiration
    // if nft.expiration.is_expired(&env.block) == true {
    //     return Err(ContractError::Expired {});
    // }
    // remove NFT from list
    LIST.remove(deps.storage, token_id.clone());
    // transfer ownership of NFT to buyer

    // transfer money to seller
    Ok(Response::new()
        // Send funds to the original owner.
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: nft.owner.clone(),
            amount: vec![info.funds[0].clone()],
        }))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: nft.contract_address,
            msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: token_id.clone(),
            })?,
            funds: vec![],
        }))
        .add_attribute("Action", "Buy")
        .add_attribute("Buyer", info.sender.to_string())
        .add_attribute("Seller", nft.owner)
        .add_attribute("NFT", token_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetListing { token_id } => to_binary(&query_listing(deps, token_id)?),
    }
}
fn query_listing(deps: Deps, token_id: String) -> StdResult<ListingResponse> {
    let nft = LIST.load(deps.storage, token_id)?;
    Ok(ListingResponse { nft })
}
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{
//         mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
//     };
//     use cosmwasm_std::{coin, coins, from_binary, CosmosMsg, WasmMsg};

//     const CONTRACT_NAME: &str = "Marketplace";
//     const CONTRACT_VERSION: &str = "The First";

//     #[test]

//     fn proper_initialization() {
//         let mut deps = mock_dependencies();

//         // Instantiate an empty contract
//         let instantiate_msg = InstantiateMsg {};
//         let info = mock_info("anyone", &[]);
//         let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
//         assert_eq!(0, res.messages.len());
//     }

//     #[test]

//     fn proper_sell() {
//         let mut deps = mock_dependencies();
//         let mut depss = mock_dependencies();
//         const CONTRACT: &str = "HEY";

//         let info = mock_info("anyone", &[]);
//         let msg = ExecuteMsg::Sell {
//             token_id: String::from("LINK"),
//             contract_address: String::from(CONTRACT),
//             price: coin(100, "UST"),
//             expiration: cw721::Expiration::AtHeight(90000),
//         };
//         let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), InstantiateMsg {}).unwrap();
//         // sender has to be owner of NFT

//         let err = execute(deps.as_mut(), depss.as_mut(), mock_env(), info, msg).unwrap_err();
//         assert_eq!(err, ContractError::Unauthorized {});
//     }
// }
