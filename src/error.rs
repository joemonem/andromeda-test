use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },

    #[error("Invalid Amount")]
    InvalidAmount {},

    #[error("Invalid Denomination")]
    InvalidDenomination {},

    #[error("Expired")]
    Expired {},

    #[error("Already Listed")]
    AlreadyListed {},

    #[error("Not Listed")]
    NotListed {},

    #[error("Unsurpassed Highest Bid")]
    UnsurpassedHighestBid {},

    #[error("Ongoing Auction")]
    OngoingAuction {},

    #[error("Unapproved")]
    Unapproved {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
