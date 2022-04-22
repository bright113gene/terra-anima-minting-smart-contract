use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{
    BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    Uint128,
};

use cw2::set_contract_version;
use cw721::{
    ContractInfoResponse, CustomMsg, Cw721Execute, Cw721ReceiveMsg, Expiration, NumTokensResponse,
};

use crate::error::ContractError;
use crate::msg::*;
use crate::state::*;
use crate::traits::*;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

impl<'a> Cw721ExtendedContract<'a> {
    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<Extension>,
    ) -> Result<Response, ContractError> {
        match msg {
            ExecuteMsg::FreeMint(msg) => self.execute_free_mint(deps, env, info, msg),
            ExecuteMsg::Withdraw {} => self.execute_withdraw(deps, env, info),
            ExecuteMsg::SetBaseUri { base_uri } => {
                self.execute_set_base_uri(deps, env, info, base_uri)
            }
            ExecuteMsg::SetArtReveal { art_reveal } => {
                self.execute_set_art_reveal(deps, env, info, art_reveal)
            }
            ExecuteMsg::Sign {} => self.execute_sign(deps, env, info),
            _ => Cw721ExtendedContract::default()._execute(deps, env, info, msg),
        }
    }
}

impl<'a> Cw721ExtendedExecute<Extension> for Cw721ExtendedContract<'a> {
    fn execute_withdraw(
        &self,
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
    ) -> Result<Response, ContractError> {
        let team = self.team.load(deps.storage)?;
        let pro = self.pro.load(deps.storage)?;
        let treas = self.treas.load(deps.storage)?;

        let current_uluna_amount = deps
            .querier
            .query_balance(env.contract.address.to_string(), "uluna")?
            .amount;

        let team_portion = current_uluna_amount * Uint128::from(30u128) / Uint128::from(100u128);
        let pro_portion = current_uluna_amount * Uint128::from(14u128) / Uint128::from(100u128);
        let treas_portion = current_uluna_amount * Uint128::from(56u128) / Uint128::from(100u128);

        let mut messages: Vec<CosmosMsg> = vec![];
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: team.to_string(),
            amount: vec![Coin {
                denom: String::from("uluna"),
                amount: team_portion,
            }],
        }));
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: pro.to_string(),
            amount: vec![Coin {
                denom: String::from("uluna"),
                amount: pro_portion,
            }],
        }));
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: treas.to_string(),
            amount: vec![Coin {
                denom: String::from("uluna"),
                amount: treas_portion,
            }],
        }));
        Ok(Response::new().add_messages(messages))
    }

    fn execute_set_base_uri(
        &self,
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _uri: String,
    ) -> Result<Response, ContractError> {
        self.base_uri.save(deps.storage, &_uri)?;

        Ok(Response::new()
            .add_attribute("action", "set_base_uri")
            .add_attribute("base_uri", &_uri))
    }

    fn execute_set_art_reveal(
        &self,
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        art_reveal: bool,
    ) -> Result<Response, ContractError> {
        self.is_on_reveal.save(deps.storage, &art_reveal)?;

        Ok(Response::new()
            .add_attribute("action", "set_art_reveal")
            .add_attribute("base_uri", &art_reveal.to_string()))
    }

    fn execute_free_mint(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: FreeMintMsg<Extension>,
    ) -> Result<Response, ContractError> {
        let freemint_count: u64 = self
            .freemint_count
            .may_load(deps.storage)?
            .unwrap_or_default();

        let minter = self.minter.load(deps.storage)?;
        if info.sender != minter {
            return Err(ContractError::Unauthorized {});
        }

        let free_drop = info.sender == minter && freemint_count < 50;

        if !free_drop {
            return Err(ContractError::Unauthorized {});
        };

        // create the token
        let token = TokenInfo {
            owner: deps.api.addr_validate(&msg.owner)?,
            approvals: vec![],
            token_uri: None,
            extension: msg.extension,
        };

        let token_id: &str = &(freemint_count + 3001).to_string()[..];
        self.tokens
            .update(deps.storage, token_id, |old| match old {
                Some(_) => Err(ContractError::Claimed {}),
                None => Ok(token),
            })?;

        self.wallet_balance
            .update(deps.storage, &info.sender, |old| match old {
                None => Err(ContractError::Claimed {}),
                Some(old_balance) => Ok(old_balance + 1),
            });

        self.freemint_count
            .save(deps.storage, &(freemint_count + 1))?;

        Ok(Response::new()
            .add_attribute("action", "free_mint")
            .add_attribute("minter", info.sender)
            .add_attribute("owner", msg.owner)
            .add_attribute("token_id", token_id))
    }

    fn execute_sign(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
    ) -> Result<Response, ContractError> {
        let team = self.team.load(deps.storage)?;
        let pro = self.pro.load(deps.storage)?;
        let treas = self.treas.load(deps.storage)?;

        if info.sender == team || info.sender == pro || info.sender == treas {
            self.cw3_signature
                .save(deps.storage, &info.sender, &(true))?;
            return Ok(Response::new()
                .add_attribute("action", "sign_for_withdraw")
                .add_attribute("signer", info.sender)
                .add_attribute("time", &env.block.time.seconds().to_string()));
        } else {
            return Err(ContractError::Unauthorized {});
        }
    }
}
