use cosmwasm_schema::cw_serde;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub manager: String, // an internal admin or manager of the contract, not the same as the --admin flag passed during the instantiation of the contract
    pub allowed_mint_addresses: Vec<String>, // addresses that are allowed to pass the ExecuteMsg::Mint to this contract. This would be your contract's address
    pub denoms: Vec<String>, // denomination of the native token that this contract manages the minting of
}

pub const STATE: Item<Config> = Item::new("config");
