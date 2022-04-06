use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Querier, QueryRequest, Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw721::{
    Approval, ApprovedForAllResponse, Cw721ExecuteMsg, Cw721QueryMsg, Expiration, OwnerOfResponse,
};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    AuctionListingResponse, ExecuteMsg, HighestBidderResponse, InstantiateMsg, ListingResponse,
    ListingsResponse, QueryMsg,
};
use crate::state::{AuctionNft, Bidder, AUCTION_LIST, HIGHEST_BIDDER, LIST, NFT};

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
fn get_token_approval(
    deps: Deps,
    contract_address: String,
    owner: String,
) -> Result<Vec<Approval>, ContractError> {
    let res: ApprovedForAllResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: contract_address,
            msg: to_binary(&Cw721QueryMsg::ApprovedForAll {
                owner,
                include_expired: None,
                start_after: None,
                limit: None,
            })?,
        }))?;
    Ok(res.operators)
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
        ExecuteMsg::Auction {
            token_id,
            contract_address,
            starting_price,
            expiration,
        } => execute_auction(
            deps,
            env,
            info,
            token_id,
            contract_address,
            starting_price,
            expiration,
        ),
        ExecuteMsg::Bid { token_id } => execute_bid(deps, env, info, token_id),
        ExecuteMsg::Claim { token_id } => execute_claim(deps, env, info, token_id),
    }
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let nft = AUCTION_LIST.load(deps.storage, token_id.clone())?;
    // check if expired
    if nft.expiration.is_expired(&env.block) == false {
        return Err(ContractError::OngoingAuction {});
    }
    // get highest bid
    let highest_bid = HIGHEST_BIDDER.load(deps.storage, token_id.clone())?;
    let amount = highest_bid.bid;
    let winner = highest_bid.address;
    // remove nft from auction list
    AUCTION_LIST.remove(deps.storage, token_id.clone());
    // remove highest bidder
    HIGHEST_BIDDER.remove(deps.storage, token_id.clone());

    Ok(Response::new()
        // Send funds to the original owner.
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: nft.owner.clone(),
            amount: vec![amount],
        }))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: nft.contract_address,
            msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: winner,
                token_id: token_id.clone(),
            })?,
            funds: vec![],
        }))
        .add_attribute("Action", "Buy")
        .add_attribute("Buyer", info.sender.to_string())
        .add_attribute("Seller", nft.owner)
        .add_attribute("NFT", token_id))
}

pub fn execute_auction(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    contract_address: String,
    starting_price: Coin,
    expiration: Expiration,
) -> Result<Response, ContractError> {
    // check if NFT is already listed
    if AUCTION_LIST.has(deps.storage, token_id.clone()) == true {
        return Err(ContractError::AlreadyListed {});
    }
    // retrieve NFT owner
    let owner = get_token_owner(deps.as_ref(), token_id.clone(), contract_address.clone())?;
    // check if sender is owner
    if info.sender.to_string() != owner {
        return Err(ContractError::Unauthorized {});
    }
    // valid starting price
    if starting_price.amount <= Uint128::from(0 as u32) {
        return Err(ContractError::InvalidAmount {});
    }
    // valid denomination
    if starting_price.denom != "uusd".to_string() {
        return Err(ContractError::InvalidDenomination {});
    }
    // valid expiration
    if expiration.is_expired(&env.block) == true {
        return Err(ContractError::Expired {});
    }
    // check if the marketplace contract has approval
    let marketplace_address = Approval {
        spender: env.contract.address.to_string(),
        expires: expiration,
    };
    let approvals = get_token_approval(deps.as_ref(), contract_address.clone(), owner.clone())?;
    let presence = approvals.contains(&marketplace_address);
    if presence == false {
        return Err(ContractError::Unapproved {});
    }

    // add the nft's components
    let nft = AuctionNft {
        token_id: token_id.clone(),
        owner,
        contract_address,
        starting_price: starting_price.clone(),
        expiration,
    };
    // add to auction list
    AUCTION_LIST.save(deps.storage, token_id.clone(), &nft)?;

    let res = Response::new()
        .add_attribute("action", "auction")
        .add_attribute("ID", token_id)
        .add_attribute("expires", expiration.to_string())
        .add_attribute("starting_price", starting_price.to_string().clone());

    Ok(res)
}
pub fn execute_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    // check if nft is in the auction list
    if AUCTION_LIST.has(deps.storage, token_id.clone()) == false {
        return Err(ContractError::NotListed {});
    }
    // check expiry
    let nft = AUCTION_LIST.load(deps.storage, token_id.clone())?;
    if nft.expiration.is_expired(&env.block) == true {
        return Err(ContractError::Expired {});
    }
    // Check for correct funds
    if info.funds[0].amount <= Uint128::from(0 as u32) {
        return Err(ContractError::InvalidAmount {});
    }
    // check for correct denom
    if info.funds[0].denom != "uusd".to_string() {
        return Err(ContractError::InvalidDenomination {});
    }
    // get highest bid
    let highest_bid = HIGHEST_BIDDER.load(deps.storage, token_id.clone())?;
    let amount = highest_bid.bid.amount;
    // check if the bid surpasses the current highest bid
    if info.funds[0].amount <= amount {
        return Err(ContractError::UnsurpassedHighestBid {});
    }
    let new_highest_bidder = Bidder {
        address: info.sender.to_string(),
        bid: info.funds[0].clone(),
    };
    // replace the previous highest bid with the new one
    HIGHEST_BIDDER.remove(deps.storage, token_id.clone());
    HIGHEST_BIDDER.save(deps.storage, token_id.clone(), &new_highest_bidder)?;

    Ok(Response::new()
        .add_attribute("Action", "Bid")
        .add_attribute("Bidder", info.sender.to_string())
        .add_attribute("Seller", nft.owner)
        .add_attribute("NFT", token_id))
}

pub fn execute_sell(
    deps: DepsMut,
    env: Env,
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
    // valid expiration
    if expiration.is_expired(&env.block) == true {
        return Err(ContractError::Expired {});
    }

    // transfer permission to the marketplace smart contract by sending a message to the NFT smart contract
    // check if the marketplace contract has approval
    let marketplace_address = Approval {
        spender: env.contract.address.to_string(),
        expires: expiration,
    };
    let approvals = get_token_approval(deps.as_ref(), contract_address.clone(), owner.clone())?;
    let presence = approvals.contains(&marketplace_address);
    if presence == false {
        return Err(ContractError::Unapproved {});
    }

    // let presence: String = approvals
    //     .clone()
    //     .into_iter()
    //     .map(|x| x.spender)
    //     .filter(|x| *x == marketplace_address)
    //     .collect();

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
    if info.funds[0].denom != "uusd".to_string() {
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
        // QueryMsg::GetListings {} => to_binary(&query_listings(deps)?),
        QueryMsg::GetAuctionListing { token_id } => {
            to_binary(&query_auction_listing(deps, token_id)?)
        }
        QueryMsg::GetHighestBidder { token_id } => {
            to_binary(&query_highest_bidder(deps, token_id)?)
        }
    }
}
fn query_listing(deps: Deps, token_id: String) -> StdResult<ListingResponse> {
    let nft = LIST.load(deps.storage, token_id)?;
    Ok(ListingResponse { nft })
}
// fn query_listings(deps: Deps) -> StdResult<ListingsResponse> {
//     let nft_list = LIST.range(deps.storage, min, None, order)
// }
fn query_auction_listing(deps: Deps, token_id: String) -> StdResult<AuctionListingResponse> {
    let auction_nft = AUCTION_LIST.load(deps.storage, token_id)?;
    Ok(AuctionListingResponse { auction_nft })
}
fn query_highest_bidder(deps: Deps, token_id: String) -> StdResult<HighestBidderResponse> {
    let highest_bidder = HIGHEST_BIDDER.load(deps.storage, token_id)?;
    Ok(HighestBidderResponse {
        bidder: highest_bidder,
    })
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
