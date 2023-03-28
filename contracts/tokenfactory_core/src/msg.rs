use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    // the manager of the contract is the one who can transfer the admin to another address
    // Typically this should be a multisig or a DAO (https://daodao.zone/)
    // Default is the contract initializer
    pub manager: Option<String>, // internal manager of the contract. Different from the external admin passed in the --admin flag from the cli during instantiation.
    pub allowed_mint_addresses: Vec<String>, // addresses allowed to pass the ExecuteMsg::Mint to this contract. This would be your contract's address.

    // We can manage multiple denoms
    pub denoms: Vec<String>, // ex: factory/juno1xxx/test
}

pub use tokenfactory_types::msg::ExecuteMsg;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    GetConfig {},
    // #[returns(Vec<Denom>)]
    // GetDenoms {},
}
