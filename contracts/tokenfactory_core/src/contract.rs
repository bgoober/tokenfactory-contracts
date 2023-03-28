#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::helpers::{
    is_contract_manager, is_whitelisted, mint_factory_token_messages, pretty_denoms_output,
};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, STATE};

use token_bindings::{TokenFactoryMsg, TokenMsg};

// Conditionally adds an entry point attribute to the function, depending on whether or not the "library" feature is enabled.
#[cfg_attr(not(feature = "library"), entry_point)]
// The `instantiate` function is called once when the contract is first instantiated on the blockchain.
pub fn instantiate(
    // A mutable reference to the `Deps` struct, which provides access to the contract's dependencies.
    deps: DepsMut,
    // The current `Env` struct, which provides information about the blockchain environment.
    _env: Env,
    // Information about the current `Message`, such as the sender and any attached tokens.
    _info: MessageInfo,
    // The `InstantiateMsg` struct, which contains any configuration options passed in by the contract deployer.
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Set the contract version in the contract's storage, using the `set_contract_version` helper function.
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Loop through each denomination in the `msg` parameter, and check that it starts with the string "factory/".
    for d in msg.denoms.iter() {
        if !d.starts_with("factory/") {
            // If a denomination does not start with "factory/", return an error with an `InvalidDenom` variant of the `ContractError` enum.
            return Err(ContractError::InvalidDenom {
                denom: d.clone(),
                message: "Denom must start with 'factory/'".to_string(),
            });
        }
    }

    // Parse the manager address from the `msg` parameter, or use the sender address if none is provided.
    let manager = deps
        .api
        .addr_validate(&msg.manager.unwrap_or_else(|| _info.sender.to_string()))?;

    // Create a new `Config` struct using the `manager` address and other configuration options from the `msg` parameter.
    let config = Config {
        manager: manager.to_string(),
        allowed_mint_addresses: msg.allowed_mint_addresses,
        denoms: msg.denoms,
    };
    // Save the `config` struct to the contract's storage using the `STATE` global state wrapper.
    STATE.save(deps.storage, &config)?;

    // Return a new `Response` struct with an "method" attribute set to "instantiate".
    Ok(Response::new().add_attribute("method", "instantiate"))
}

// Conditionally adds an entry point attribute to the function, depending on whether or not the "library" feature is enabled.
#[cfg_attr(not(feature = "library"), entry_point)]
// The `execute` function is called whenever the contract receives a new message.
pub fn execute(
    // A mutable reference to the `Deps` struct, which provides access to the contract's dependencies.
    deps: DepsMut,
    // The current `Env` struct, which provides information about the blockchain environment.
    env: Env,
    // Information about the current `Message`, such as the sender and any attached tokens.
    info: MessageInfo,
    // The `ExecuteMsg` enum, which contains the specific action to be executed by the contract.
    msg: ExecuteMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Match the `msg` parameter to one of several possible variants of the `ExecuteMsg` enum.
    match msg {
        // If the `msg` parameter is an `ExecuteMsg::Burn` variant, call the `execute_burn` function.
        // This function is permissionless, meaning anyone can call it.
        ExecuteMsg::Burn {} => execute_burn(deps, env, info),

        // If the `msg` parameter is an `ExecuteMsg::Mint` variant, call the `execute_mint` function.
        // This function is only callable by addresses on the contract's whitelist.
        ExecuteMsg::Mint { address, denom } => execute_mint(deps, info, address, denom),

        // If the `msg` parameter is an `ExecuteMsg::TransferAdmin` variant, call the `execute_transfer_admin` function.
        // This function is only callable by the contract manager and allows transferring the minting admin rights for a given denom to a new address.
        ExecuteMsg::TransferAdmin { denom, new_address } => {
            execute_transfer_admin(deps, info, denom, new_address)
        }

        // If the `msg` parameter is an `ExecuteMsg::AddWhitelist` variant, update the whitelist of allowed mint addresses.
        // This function is only callable by the contract manager.
        ExecuteMsg::AddWhitelist { addresses } => {
            // Load the current contract state from storage.
            let state = STATE.load(deps.storage)?;
            // Ensure that the sender is the contract manager, using the `is_contract_manager` helper function.
            is_contract_manager(state.clone(), info.sender)?;

            // Add new addresses to the whitelist if they are not already present.
            let mut updated = state.allowed_mint_addresses;
            for new in addresses {
                if !updated.contains(&new) {
                    updated.push(new);
                }
            }

            // Update the contract state with the new whitelist.
            STATE.update(deps.storage, |mut state| -> StdResult<_> {
                state.allowed_mint_addresses = updated;
                Ok(state)
            })?;

            // Return a new `Response` struct with an "method" attribute set to "add_whitelist".
            Ok(Response::new().add_attribute("method", "add_whitelist"))
        }

        ExecuteMsg::RemoveWhitelist { addresses } => { // Handles RemoveWhitelist message
            let state = STATE.load(deps.storage)?; // Loads the current state of the contract from the storage
            is_contract_manager(state.clone(), info.sender)?; // Checks whether the sender is the contract manager

            let mut updated = state.allowed_mint_addresses; // Create a mutable copy of the allowed_mint_addresses from state
            for remove in addresses { // Loops through each address to remove
                updated.retain(|a| a != &remove); // Removes the current address from the updated list if it matches the current address in the loop
            }

            STATE.update(deps.storage, |mut state| -> StdResult<_> { // Updates the state in storage
                state.allowed_mint_addresses = updated; // Updates the allowed_mint_addresses field in state with the updated list
                Ok(state) // Returns the updated state
            })?;

            Ok(Response::new().add_attribute("method", "remove_whitelist")) // Returns a new response with an added "method" attribute to indicate that the RemoveWhitelist message has been successfully executed
        }


        ExecuteMsg::AddDenom { denoms } => { // Handle AddDenom message

            // Load the current state of the contract from storage
            let state = STATE.load(deps.storage)?;

            // Check whether the sender is the contract manager
            is_contract_manager(state.clone(), info.sender)?;

            // Create a mutable copy of the denoms from state
            let mut updated_denoms = state.denoms;

            // Loop through each new denom to add
            for new in denoms {

                // If the current new denom is not already in updated_denoms
                if !updated_denoms.contains(&new) {

                    // Add the current new denom to updated_denoms
                    updated_denoms.push(new);
                }
            }

            // Update the denoms in the state with updated_denoms
            STATE.update(deps.storage, |mut state| -> StdResult<_> {
                state.denoms = updated_denoms;
                Ok(state)
            })?;

            // Create a new Response object and add an attribute to signify that the method called was add_denom
            Ok(Response::new().add_attribute("method", "add_denom"))
        }

        ExecuteMsg::RemoveDenom { denoms } => { // Handles RemoveDenom message
            let state = STATE.load(deps.storage)?; // Loads the current state of the contract from the storage
            is_contract_manager(state.clone(), info.sender)?; // Checks whether the sender is the contract manager

            let mut updated_denoms = state.denoms; // Create a mutable copy of the denoms from state
            for remove in denoms { // Loops through each denom to remove
                updated_denoms.retain(|a| a != &remove); // Remove the current denom from updated_denoms
            }

            STATE.update(deps.storage, |mut state| -> StdResult<_> {
                state.denoms = updated_denoms;
                Ok(state)
            })?; // Updates the storage with the new state of the contract
            Ok(Response::new().add_attribute("method", "remove_denom")) // Returns a success response with the method name
        }
    }
}


pub fn execute_transfer_admin(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    new_addr: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let state = STATE.load(deps.storage)?; // Load the current state of the contract from storage
    is_contract_manager(state.clone(), info.sender)?; // Verify that the sender is the contract manager

    // Check if the denom is already present in the contract state
    // it is possible to transfer admin in without adding to contract state. So devs need a way to reclaim admin without adding it to denoms state
    let state_denom: Option<&String> = state.denoms.iter().find(|d| d.to_string() == denom);

    if state_denom.is_some() { // If the denom is present in the contract state
        // Remove it from the state by filtering out the given denom from the denoms vector
        let updated_state: Vec<String> = state
            .denoms
            .iter()
            .filter(|d| d.to_string() != *state_denom.unwrap())
            .map(|d| d.to_string())
            .collect();

        STATE.update(deps.storage, |mut state| -> StdResult<_> {
            state.denoms = updated_state;
            Ok(state)
        })?;
    }

    // Prepare a message to change the admin address of the given denom
    let msg = TokenMsg::ChangeAdmin {
        denom: denom.to_string(),
        new_admin_address: new_addr.to_string(),
    };

    // Return a response containing the message and some attributes
    Ok(Response::new()
        .add_attribute("method", "execute_transfer_admin")
        .add_attribute("new_admin", new_addr)
        .add_message(msg))
}


pub fn execute_mint(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    denoms: Vec<Coin>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let state = STATE.load(deps.storage)?; // Loads the current state of the contract from storage

    is_whitelisted(state, info.sender)?; // Checks whether the sender is whitelisted

    let mint_msgs: Vec<TokenMsg> = mint_factory_token_messages(&address, &denoms)?; // Generates a vector of TokenMsg that include the messages to send to other contracts to mint tokens.

    Ok(Response::new()
        .add_attribute("method", "execute_mint") // Adds the method executed to the response attributes
        .add_attribute("to_address", address) // Adds the address of the receiver to the response attributes
        .add_attribute("denoms", pretty_denoms_output(&denoms)) // Adds the denoms to be minted to the response attributes
        .add_messages(mint_msgs)) // Adds the messages generated to mint tokens to the response messages
}


pub fn execute_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // Anyone can burn funds since they have to send them in.
    if info.funds.is_empty() {
        return Err(ContractError::InvalidFunds {});
    }

    let state = STATE.load(deps.storage)?;

    // Partition funds into those with factory-denoms and those without
    let (factory_denoms, send_back): (Vec<Coin>, Vec<Coin>) = info
        .funds
        .iter()
        .cloned()
        .partition(|coin| state.denoms.iter().any(|d| *d == coin.denom));

    // Create burn messages for all funds with factory-denoms
    let burn_msgs: Vec<TokenMsg> = factory_denoms
        .iter()
        .map(|coin| TokenMsg::BurnTokens {
            denom: coin.denom.clone(),
            amount: coin.amount,
            burn_from_address: env.contract.address.to_string(),
        })
        .collect();

    // Create message to send remaining funds back to the sender
    let bank_return_msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: send_back,
    };

    Ok(Response::new()
        .add_attribute("method", "execute_burn")
        .add_message(bank_return_msg)
        .add_messages(burn_msgs))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => {
            let state = STATE.load(deps.storage)?;
            to_binary(&state)
        }
    }
}
