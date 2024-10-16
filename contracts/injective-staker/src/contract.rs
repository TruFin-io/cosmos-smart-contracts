#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, Addr, Binary, Coin, Deps, DepsMut, Env, Event, MessageInfo, Response,
    StakingMsg, StdResult, Uint128, Uint256, Uint512,
};
use cw2::set_contract_version;
use cw20::{LogoInfo, MarketingInfoResponse};
use cw20_base::contract::{
    execute_burn, execute_mint, execute_send, execute_transfer, query_balance,
    query_marketing_info, query_token_info,
};
use cw20_base::state::{MinterData, TokenInfo, MARKETING_INFO, TOKEN_INFO};
use execute::{set_distribution_fee, set_fee, set_min_deposit};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, GetDistributionAmountsResponse, GetSharePriceResponse, GetStakerInfoResponse,
    InstantiateMsg, QueryMsg,
};
use crate::state::{
    allocations, Allocation, DistributionInfo, StakerInfo, Validator, ValidatorState, CLAIMS,
    CONTRACT_REWARDS, DEFAULT_VALIDATOR, IS_PAUSED, OWNER, STAKER_INFO, VALIDATORS,
};
use crate::{whitelist, FEE_PRECISION, INJ, ONE_INJ, SHARE_PRICE_SCALING_FACTOR, UNBONDING_PERIOD};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:injective-staker";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let owner_addr = info.sender;
    let treasury_addr = deps.api.addr_validate(msg.treasury.as_str()).unwrap();
    let default_validator_addr = msg.default_validator;

    let staker_info = StakerInfo {
        treasury: treasury_addr,
        fee: 0,
        distribution_fee: 0,
        min_deposit: ONE_INJ,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STAKER_INFO.save(deps.storage, &staker_info)?;

    DEFAULT_VALIDATOR.save(deps.storage, &default_validator_addr)?;
    OWNER.save(deps.storage, &owner_addr)?;
    IS_PAUSED.save(deps.storage, &false)?;
    CONTRACT_REWARDS.save(deps.storage, &Uint128::zero())?;

    // store token info
    let data = TokenInfo {
        name: "TruINJ".to_string(),
        symbol: "TRUINJ".to_string(),
        decimals: 18,
        total_supply: Uint128::zero(),
        mint: Some(MinterData {
            minter: env.contract.address,
            cap: None,
        }),
    };
    TOKEN_INFO.save(deps.storage, &data)?;

    // store marketing info
    let marketing_info = MarketingInfoResponse {
        project: Some("TruFin".to_string()),
        description: Some("TruFin's liquid staking token".to_string()),
        logo: Some(LogoInfo::Url(
            "https://trufin-public-assets.s3.eu-west-2.amazonaws.com/truINJ-logo.svg".to_string(),
        )),
        marketing: Some(owner_addr.clone()),
    };
    MARKETING_INFO.save(deps.storage, &marketing_info)?;

    // store validator
    VALIDATORS.save(
        deps.storage,
        &default_validator_addr,
        &Validator {
            state: ValidatorState::ENABLED,
        },
    )?;

    Ok(Response::new().add_event(
        Event::new("instantiated")
            .add_attribute("owner", owner_addr)
            .add_attribute("default_validator", default_validator_addr)
            .add_attribute("treasury", msg.treasury)
            .add_attribute("token_name", "TruINJ"),
    ))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetFee { new_fee } => set_fee(deps, info.sender, new_fee),
        ExecuteMsg::SetDistributionFee {
            new_distribution_fee,
        } => set_distribution_fee(deps, info.sender, new_distribution_fee),
        ExecuteMsg::SetMinimumDeposit { new_min_deposit } => {
            set_min_deposit(deps, info.sender, new_min_deposit)
        }
        ExecuteMsg::SetTreasury { new_treasury_addr } => {
            execute::set_treasury(deps, info.sender, new_treasury_addr)
        }
        ExecuteMsg::SetDefaultValidator {
            new_default_validator_addr,
        } => execute::set_default_validator(deps, info.sender, new_default_validator_addr),
        ExecuteMsg::Transfer { recipient, amount } => {
            Ok(execute_transfer(deps, env, info, recipient, amount)?)
        }
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(execute_send(deps, env, info, contract, amount, msg)?),
        ExecuteMsg::Stake {} => execute::stake(deps, env, info),
        ExecuteMsg::StakeToSpecificValidator { validator_addr } => {
            execute::stake_to_specific_validator(deps, env, info, validator_addr)
        }
        ExecuteMsg::Unstake { amount } => execute::unstake(deps, env, info, amount.u128()),
        ExecuteMsg::UnstakeFromSpecificValidator {
            validator_addr,
            amount,
        } => {
            execute::unstake_from_specific_validator(deps, env, info, validator_addr, amount.u128())
        }
        ExecuteMsg::Claim {} => execute::claim(deps, env, info),
        ExecuteMsg::SetPendingOwner { new_owner } => {
            execute::set_pending_owner(deps, info.sender, new_owner)
        }
        ExecuteMsg::ClaimOwnership {} => execute::claim_ownership(deps, info.sender),
        ExecuteMsg::AddValidator { validator } => {
            execute::add_validator(deps, info.sender, validator)
        }
        ExecuteMsg::DisableValidator { validator } => {
            execute::disable_validator(deps, info.sender, validator)
        }
        ExecuteMsg::EnableValidator { validator } => {
            execute::enable_validator(deps, info.sender, validator)
        }
        ExecuteMsg::Pause => execute::pause(deps, info.sender),
        ExecuteMsg::Unpause => execute::unpause(deps, info.sender),

        ExecuteMsg::AddAgent { agent } => whitelist::add_agent(deps, info.sender, agent),
        ExecuteMsg::RemoveAgent { agent } => whitelist::remove_agent(deps, info.sender, agent),
        ExecuteMsg::AddUserToWhitelist { user } => {
            whitelist::add_user_to_whitelist(deps, info.sender, user)
        }
        ExecuteMsg::AddUserToBlacklist { user } => {
            whitelist::add_user_to_blacklist(deps, info.sender, user)
        }
        ExecuteMsg::ClearUserStatus { user } => {
            whitelist::clear_user_status(deps, info.sender, user)
        }
        ExecuteMsg::CompoundRewards => execute::compound_rewards(deps, env),
        ExecuteMsg::Restake {
            amount,
            validator_addr,
        } => execute::_restake(deps, env, info.sender, amount, validator_addr),
        ExecuteMsg::Allocate { recipient, amount } => {
            execute::allocate(deps, env, info.sender, recipient, amount)
        }
        ExecuteMsg::Deallocate { recipient, amount } => {
            execute::deallocate(deps, info.sender, recipient, amount)
        }
        ExecuteMsg::DistributeRewards { recipient, in_inj } => {
            execute::distribute_rewards(deps, env, info, recipient, in_inj)
        }
        #[cfg(any(test, feature = "test"))]
        ExecuteMsg::TestAllocate { recipient, amount } => {
            test_allocate(deps, env, info.sender, recipient, amount)
        }
    }
}

pub mod execute {
    use cosmwasm_std::{BankMsg, DistributionMsg, WasmMsg};
    use query::{get_share_price, get_total_allocated};

    use super::*;

    use crate::FEE_PRECISION;

    use crate::state::{Allocation, IS_PAUSED, PENDING_OWNER};

    pub fn set_fee(deps: DepsMut, sender: Addr, new_fee: u16) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender)?;

        ensure!(new_fee < FEE_PRECISION, ContractError::FeeTooLarge);

        let old_fee = STAKER_INFO.load(deps.storage)?.fee;

        STAKER_INFO.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.fee = new_fee;
            Ok(state)
        })?;

        Ok(Response::new().add_event(
            Event::new("set_fee")
                .add_attribute("old_fee", old_fee.to_string())
                .add_attribute("new_fee", new_fee.to_string()),
        ))
    }

    pub fn set_distribution_fee(
        deps: DepsMut,
        sender: Addr,
        new_distribution_fee: u16,
    ) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender)?;

        ensure!(
            new_distribution_fee < FEE_PRECISION,
            ContractError::FeeTooLarge
        );

        let old_distribution_fee = STAKER_INFO.load(deps.storage)?.distribution_fee;

        STAKER_INFO.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.distribution_fee = new_distribution_fee;
            Ok(state)
        })?;

        Ok(Response::new().add_event(
            Event::new("set_distribution_fee")
                .add_attribute("old_distribution_fee", old_distribution_fee.to_string())
                .add_attribute("new_distribution_fee", new_distribution_fee.to_string()),
        ))
    }

    pub fn set_min_deposit(
        deps: DepsMut,
        sender: Addr,
        new_min_deposit: Uint128,
    ) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender)?;

        // ensure!(new_min_deposit.u128() >= ONE_INJ, ContractError::MinimumDepositTooSmall);

        let old_min_deposit = STAKER_INFO.load(deps.storage)?.min_deposit;

        STAKER_INFO.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.min_deposit = new_min_deposit.into();
            Ok(state)
        })?;

        Ok(Response::new().add_event(
            Event::new("set_min_deposit")
                .add_attribute("old_min_deposit", old_min_deposit.to_string())
                .add_attribute("new_min_deposit", new_min_deposit.to_string()),
        ))
    }

    pub fn set_treasury(
        deps: DepsMut,
        sender: Addr,
        new_treasury_addr: Addr,
    ) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender)?;

        let validated_addr = deps.api.addr_validate(new_treasury_addr.as_str()).unwrap();

        let old_treasury_addr = STAKER_INFO.load(deps.storage)?.treasury;

        STAKER_INFO.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.treasury = validated_addr.clone();
            Ok(state)
        })?;

        Ok(Response::new().add_event(
            Event::new("set_treasury")
                .add_attribute("new_treasury_addr", validated_addr.to_string())
                .add_attribute("old_treasury_addr", old_treasury_addr.to_string()),
        ))
    }

    pub fn set_default_validator(
        deps: DepsMut,
        sender: Addr,
        new_default_validator_addr: Addr,
    ) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender)?;
        check_validator(deps.as_ref(), new_default_validator_addr.clone())?;

        let old_default_validator_addr = DEFAULT_VALIDATOR.load(deps.storage)?;

        DEFAULT_VALIDATOR.save(deps.storage, &new_default_validator_addr)?;

        Ok(Response::new().add_event(
            Event::new("set_default_validator")
                .add_attribute(
                    "new_default_validator_addr",
                    new_default_validator_addr.into_string(),
                )
                .add_attribute(
                    "old_default_validator_addr",
                    old_default_validator_addr.into_string(),
                ),
        ))
    }

    // Stakes INJ to the default validator.
    pub fn stake(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        check_not_paused(deps.as_ref())?;
        whitelist::check_whitelisted(deps.as_ref(), info.sender.clone())?;

        let validator_addr = DEFAULT_VALIDATOR.load(deps.storage)?;

        let stake_res = internal_stake(deps, env, info, validator_addr)?;
        Ok(stake_res)
    }

    // Stakes INJ to the specified validator.
    pub fn stake_to_specific_validator(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        validator_addr: Addr,
    ) -> Result<Response, ContractError> {
        check_not_paused(deps.as_ref())?;
        whitelist::check_whitelisted(deps.as_ref(), info.sender.clone())?;

        let stake_res = internal_stake(deps, env, info, validator_addr)?;
        Ok(stake_res)
    }

    /// Unstakes a certain amount of INJ from the default validator.
    pub fn unstake(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        amount: u128,
    ) -> Result<Response, ContractError> {
        check_not_paused(deps.as_ref())?;
        whitelist::check_whitelisted(deps.as_ref(), info.sender.clone())?;

        let validator_addr = DEFAULT_VALIDATOR.load(deps.storage)?;
        let unstake_res = internal_unstake(deps, env, info, validator_addr, amount)?;
        Ok(unstake_res)
    }

    /// Unstakes a certain amount of INJ from a specific validator.
    pub fn unstake_from_specific_validator(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        validator_addr: Addr,
        amount: u128,
    ) -> Result<Response, ContractError> {
        check_not_paused(deps.as_ref())?;
        whitelist::check_whitelisted(deps.as_ref(), info.sender.clone())?;

        ensure!(
            VALIDATORS.has(deps.storage, &validator_addr),
            ContractError::ValidatorDoesNotExist
        );

        let unstake_res = internal_unstake(deps, env, info, validator_addr, amount)?;
        Ok(unstake_res)
    }

    /// Sets a pending owner. The pending owner has no contract privileges.
    pub fn set_pending_owner(
        deps: DepsMut,
        sender: Addr,
        new_owner: Addr,
    ) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender.clone())?;
        let new_owner = deps.api.addr_validate(new_owner.as_str()).unwrap();

        PENDING_OWNER.save(deps.storage, &new_owner)?;

        Ok(Response::new().add_event(
            Event::new("set_pending_owner")
                .add_attribute("current_owner", sender)
                .add_attribute("pending_owner", new_owner),
        ))
    }

    pub fn allocate(
        deps: DepsMut,
        env: Env,
        sender: Addr,
        recipient: Addr,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        check_not_paused(deps.as_ref())?;
        whitelist::check_whitelisted(deps.as_ref(), sender.clone())?;
        deps.api.addr_validate(recipient.as_str())?;

        ensure!(recipient != sender, ContractError::InvalidRecipient {});
        ensure!(
            amount.u128() >= ONE_INJ,
            ContractError::AllocationUnderOneInj {}
        );
        let share_price_response = get_share_price(deps.as_ref(), env.contract.address);
        let recipient_allocation = allocations().update(
            deps.storage,
            (sender.clone(), recipient.clone()),
            |existing| -> Result<_, ContractError> {
                existing.map_or_else(
                    || {
                        Ok(Allocation {
                            allocator: sender.clone(),
                            recipient: recipient.clone(),
                            inj_amount: amount,
                            share_price_num: share_price_response.numerator,
                            share_price_denom: share_price_response.denominator,
                        })
                    },
                    |allocation| {
                        let updated_allocation = calculate_updated_allocation(
                            &allocation,
                            amount,
                            share_price_response.numerator,
                            share_price_response.denominator,
                        )
                        .unwrap();
                        Ok(updated_allocation)
                    },
                )
            },
        )?;

        let total_allocated_response = get_total_allocated(deps.as_ref(), sender.clone())?;

        Ok(Response::new().add_event(
            Event::new("allocated")
                .add_attribute("user", sender)
                .add_attribute("recipient", recipient)
                .add_attribute("amount", amount)
                .add_attribute("total_amount", recipient_allocation.inj_amount)
                .add_attribute("share_price_num", recipient_allocation.share_price_num)
                .add_attribute("share_price_denom", recipient_allocation.share_price_denom)
                .add_attribute(
                    "total_allocated_amount",
                    total_allocated_response.total_allocated_amount,
                )
                .add_attribute(
                    "total_allocated_share_price_num",
                    total_allocated_response.total_allocated_share_price_num,
                )
                .add_attribute(
                    "total_allocated_share_price_denom",
                    total_allocated_response.total_allocated_share_price_denom,
                ),
        ))
    }

    pub fn deallocate(
        deps: DepsMut,
        sender: Addr,
        recipient: Addr,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        check_not_paused(deps.as_ref())?;
        whitelist::check_whitelisted(deps.as_ref(), sender.clone())?;

        let mut allocation = allocations()
            .load(deps.storage, (sender.clone(), recipient.clone()))
            .map_err(|_| ContractError::NoAllocationToRecipient)?;

        ensure!(
            allocation.inj_amount >= amount,
            ContractError::ExcessiveDeallocation {}
        );

        let remaining_amount = allocation.inj_amount - amount;
        if remaining_amount.is_zero() {
            allocations().replace(
                deps.storage,
                (sender.clone(), recipient.clone()),
                None,
                Some(&allocation),
            )?;
        } else {
            ensure!(
                remaining_amount.u128() >= ONE_INJ,
                ContractError::AllocationUnderOneInj {}
            );
            let old_allocation = allocation.clone();
            allocation.inj_amount = remaining_amount;
            allocations().replace(
                deps.storage,
                (sender.clone(), recipient.clone()),
                Some(&allocation),
                Some(&old_allocation),
            )?;
        }

        let total_allocated_response = get_total_allocated(deps.as_ref(), sender.clone())?;

        Ok(Response::new().add_event(
            Event::new("deallocated")
                .add_attribute("user", sender)
                .add_attribute("recipient", recipient)
                .add_attribute("amount", amount)
                .add_attribute("total_amount", remaining_amount)
                .add_attribute("share_price_num", allocation.share_price_num)
                .add_attribute("share_price_denom", allocation.share_price_denom)
                .add_attribute(
                    "total_allocated_amount",
                    total_allocated_response.total_allocated_amount,
                )
                .add_attribute(
                    "total_allocated_share_price_num",
                    total_allocated_response.total_allocated_share_price_num,
                )
                .add_attribute(
                    "total_allocated_share_price_denom",
                    total_allocated_response.total_allocated_share_price_denom,
                ),
        ))
    }

    /// Distribute allocation rewards from the caller to the specified recipient.
    pub fn distribute_rewards(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        recipient: Addr,
        in_inj: bool,
    ) -> Result<Response, ContractError> {
        check_not_paused(deps.as_ref())?;
        let distributor = info.sender.clone();
        whitelist::check_whitelisted(deps.as_ref(), distributor.clone())?;

        ensure!(
            !allocations()
                .prefix(distributor.clone())
                .is_empty(deps.storage),
            ContractError::NoAllocations
        );

        ensure!(
            allocations().has(deps.storage, (distributor.clone(), recipient.clone())),
            ContractError::NoAllocationToRecipient
        );

        let attached_inj_amount = cw_utils::may_pay(&info, INJ)?.u128();

        // get the allocation to the recipient
        let allocation =
            allocations().load(deps.storage, (distributor.clone(), recipient.clone()))?;

        // distribute rewards for the current share price
        let contract_addr = env.contract.address.clone();
        let share_price = get_share_price(deps.as_ref(), contract_addr);
        let distribution_info = match internal_distribute(
            deps.branch(),
            env,
            &distributor,
            &recipient,
            allocation,
            in_inj,
            share_price.numerator,
            share_price.denominator,
            attached_inj_amount,
        )? {
            Some(info) => info,
            // return if there are no rewards to distribute
            None => {
                let response = if attached_inj_amount > 0 {
                    Response::new().add_message(BankMsg::Send {
                        to_address: distributor.to_string(),
                        amount: vec![Coin {
                            denom: INJ.to_string(),
                            amount: attached_inj_amount.into(),
                        }],
                    })
                } else {
                    Response::new()
                };
                return Ok(response);
            }
        };

        // update the share price of the allocation
        allocations().update(
            deps.storage,
            (distributor.clone(), recipient),
            |existing| -> Result<_, ContractError> {
                let mut updated_alloc = existing.unwrap();
                updated_alloc.share_price_num = share_price.numerator;
                updated_alloc.share_price_denom = share_price.denominator;
                Ok(updated_alloc)
            },
        )?;

        let total_allocated = get_total_allocated(deps.as_ref(), distributor.clone())?;

        // prepare the response
        let mut response: Response = Response::new();

        // if the distribution is in INJ, transfer the amount to the recipient
        if distribution_info.in_inj {
            response = response.add_message(BankMsg::Send {
                to_address: distribution_info.recipient.to_string(),
                amount: vec![Coin {
                    denom: INJ.to_string(),
                    amount: distribution_info.inj_amount.into(),
                }],
            });
        }

        // if there is a refund, tranfer the amount to the distributor
        if distribution_info.refund_amount > 0 {
            response = response.add_message(BankMsg::Send {
                to_address: distributor.to_string(),
                amount: vec![Coin {
                    denom: INJ.to_string(),
                    amount: distribution_info.refund_amount.into(),
                }],
            });
        }

        Ok(response.add_event(
            Event::new("distributed-rewards")
                .add_attribute("user", distribution_info.user)
                .add_attribute("recipient", distribution_info.recipient)
                .add_attribute("user_balance", distribution_info.user_balance.to_string())
                .add_attribute(
                    "recipient_balance",
                    distribution_info.recipient_balance.to_string(),
                )
                .add_attribute(
                    "treasury_balance",
                    distribution_info.treasury_balance.to_string(),
                )
                .add_attribute("fees", distribution_info.fees.to_string())
                .add_attribute("shares", distribution_info.shares.to_string())
                .add_attribute("inj_amount", distribution_info.inj_amount.to_string())
                .add_attribute("in_inj", distribution_info.in_inj.to_string())
                .add_attribute("share_price_num", distribution_info.share_price_num)
                .add_attribute("share_price_denom", distribution_info.share_price_denom)
                .add_attribute(
                    "total_allocated_amount",
                    total_allocated.total_allocated_amount,
                )
                .add_attribute(
                    "total_allocated_share_price_num",
                    total_allocated.total_allocated_share_price_num,
                )
                .add_attribute(
                    "total_allocated_share_price_denom",
                    total_allocated.total_allocated_share_price_denom,
                ),
        ))
    }

    // Allows a user to withdraw all their expired claims.
    pub fn claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        check_not_paused(deps.as_ref())?;
        whitelist::check_whitelisted(deps.as_ref(), info.sender.clone())?;

        let contract_addr = env.contract.address.clone();
        let contract_rewards = CONTRACT_REWARDS.load(deps.storage)?;
        let total_assets = deps
            .querier
            .query_balance(contract_addr, INJ)
            .unwrap()
            .amount;
        let contract_claimed_amount = total_assets.saturating_sub(contract_rewards);

        let claimed_amount = CLAIMS.claim_tokens(
            deps.storage,
            &info.sender,
            &env.block,
            Some(contract_claimed_amount),
        )?;

        ensure!(
            claimed_amount > Uint128::zero(),
            ContractError::NothingToClaim {}
        );

        // transfer tokens to the sender
        let res = Response::new()
            .add_message(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    denom: INJ.to_string(),
                    amount: claimed_amount,
                }],
            })
            .add_event(
                Event::new("claimed")
                    .add_attribute("user", info.sender)
                    .add_attribute("amount", claimed_amount),
            );
        Ok(res)
    }

    /// Allows the pending owner to claim ownership of the contract.
    pub fn claim_ownership(deps: DepsMut, sender: Addr) -> Result<Response, ContractError> {
        let pending_owner = PENDING_OWNER
            .load(deps.storage)
            .map_err(|_| ContractError::NoPendingOwnerSet)?;

        ensure!(sender == pending_owner, ContractError::NotPendingOwner);

        let old_owner = OWNER.load(deps.storage)?;

        // set new owner
        OWNER.save(deps.storage, &sender)?;

        // remove pending owner
        PENDING_OWNER.remove(deps.storage);

        Ok(Response::new().add_event(
            Event::new("claimed_ownership")
                .add_attribute("new_owner", sender)
                .add_attribute("old_owner", old_owner),
        ))
    }

    // Adds a new validator that can be staked to.
    pub fn add_validator(
        deps: DepsMut,
        sender: Addr,
        validator_addr: Addr,
    ) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender)?;
        ensure!(
            !VALIDATORS.has(deps.storage, &validator_addr),
            ContractError::ValidatorAlreadyExists
        );

        let validator = Validator {
            state: ValidatorState::ENABLED,
        };
        VALIDATORS.save(deps.storage, &validator_addr, &validator)?;
        Ok(Response::new().add_event(
            Event::new("validator_added")
                .add_attribute("validator_address", validator_addr.to_string()),
        ))
    }

    // Enables a previously disabled validator.
    pub fn enable_validator(
        deps: DepsMut,
        sender: Addr,
        validator_addr: Addr,
    ) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender)?;
        VALIDATORS.update(
            deps.storage,
            &validator_addr,
            |validator| -> Result<_, ContractError> {
                let mut validator = validator.ok_or(ContractError::ValidatorDoesNotExist)?;

                ensure!(
                    validator.state != ValidatorState::ENABLED,
                    ContractError::ValidatorAlreadyEnabled
                );

                validator.state = ValidatorState::ENABLED;
                Ok(validator)
            },
        )?;

        Ok(Response::new().add_event(
            Event::new("validator_enabled")
                .add_attribute("validator_address", validator_addr.to_string()),
        ))
    }

    // Disables a previously enabled validator. Disabled validators cannot be staked to but stake already on the validator can be
    /// unstaked and withdrawn as normal.
    pub fn disable_validator(
        deps: DepsMut,
        sender: Addr,
        validator_addr: Addr,
    ) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender)?;
        VALIDATORS.update(
            deps.storage,
            &validator_addr,
            |validator| -> Result<_, ContractError> {
                let mut validator = validator.ok_or(ContractError::ValidatorDoesNotExist)?;

                ensure!(
                    validator.state != ValidatorState::DISABLED,
                    ContractError::ValidatorAlreadyDisabled
                );

                validator.state = ValidatorState::DISABLED;
                Ok(validator)
            },
        )?;

        Ok(Response::new().add_event(
            Event::new("validator_disabled")
                .add_attribute("validator_address", validator_addr.to_string()),
        ))
    }

    /// Pauses the contract to prevent user operations.
    pub fn pause(deps: DepsMut, sender: Addr) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender)?;
        check_not_paused(deps.as_ref())?;

        IS_PAUSED.save(deps.storage, &true)?;

        Ok(Response::new().add_event(Event::new("paused")))
    }

    /// Unpauses the contract to allow user operations.
    pub fn unpause(deps: DepsMut, sender: Addr) -> Result<Response, ContractError> {
        check_owner(deps.as_ref(), sender)?;

        // set paused to false if the contract is paused
        IS_PAUSED.update(deps.storage, |paused| -> Result<_, ContractError> {
            ensure!(paused, ContractError::NotPaused);
            Ok(false)
        })?;

        Ok(Response::new().add_event(Event::new("unpaused")))
    }

    pub fn compound_rewards(mut deps: DepsMut, env: Env) -> Result<Response, ContractError> {
        let contract_addr = env.contract.address.clone();
        let mut total_rewards = 0u128;
        let mut total_staked = 0u128;

        let mut collect_rewards_messages = Vec::new();
        let mut restake_messages = Vec::new();

        for validator in VALIDATORS.range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        {
            let (validator_addr, _) = validator?;
            if let Some(delegation) = deps
                .querier
                .query_delegation(contract_addr.clone(), validator_addr.clone())?
            {
                total_staked += delegation.amount.amount.u128();
                if let Some(reward) = delegation
                    .accumulated_rewards
                    .iter()
                    .find(|coin| coin.denom == INJ)
                {
                    total_rewards += reward.amount.u128();
                    collect_rewards_messages.push(DistributionMsg::WithdrawDelegatorReward {
                        validator: validator_addr.to_string(),
                    });
                    let msg = to_json_binary(&ExecuteMsg::Restake {
                        amount: reward.amount,
                        validator_addr: validator_addr.clone(),
                    })?;
                    restake_messages.push(WasmMsg::Execute {
                        contract_addr: contract_addr.to_string(),
                        msg,
                        funds: vec![],
                    });
                }
            }
        }

        if total_rewards == 0 {
            return Ok(Response::new());
        }

        let staker_info = STAKER_INFO.load(deps.storage)?;

        let fees: u128 = total_rewards * u128::from(staker_info.fee) / u128::from(FEE_PRECISION);
        let mut treasury_share_increase = Uint128::from(0u128);

        if fees > 0 {
            let shares_supply = TOKEN_INFO.load(deps.storage)?.total_supply;

            let contract_rewards: Uint128 = CONTRACT_REWARDS.load(deps.storage)?;

            let (share_price_num, share_price_denom) = internal_share_price(
                total_staked,
                contract_rewards.u128(),
                total_rewards,
                shares_supply.u128(),
                staker_info.fee,
            );

            treasury_share_increase =
                convert_to_shares((fees).into(), share_price_num, share_price_denom).unwrap();

            let minter_info = MessageInfo {
                sender: contract_addr,
                funds: vec![],
            };

            // mint TruINJ to the treasury
            execute_mint(
                deps.branch(),
                env,
                minter_info,
                staker_info.treasury.clone().into_string(),
                treasury_share_increase,
            )?;
        }

        let res = Response::new()
            .add_messages(collect_rewards_messages)
            .add_messages(restake_messages)
            .add_event(
                Event::new("restaked")
                    .add_attribute("amount", Uint128::from(total_rewards))
                    .add_attribute("treasury_shares_minted", treasury_share_increase)
                    .add_attribute(
                        "treasury_balance",
                        query_balance(deps.as_ref(), staker_info.treasury.into_string())?.balance,
                    ),
            );
        Ok(res)
    }

    pub fn _restake(
        deps: DepsMut,
        env: Env,
        sender: Addr,
        mut restake_amount: Uint128,
        restake_validator: Addr,
    ) -> Result<Response, ContractError> {
        ensure!(sender == env.contract.address, ContractError::Unauthorized);

        let default_validator = DEFAULT_VALIDATOR.load(deps.storage)?;
        if restake_validator == default_validator {
            // restake the contract rewards with the default validator
            let rewards = CONTRACT_REWARDS.load(deps.storage)?;
            restake_amount += rewards;
            CONTRACT_REWARDS.save(deps.storage, &Uint128::zero())?;
        }

        let res = Response::new().add_message(StakingMsg::Delegate {
            validator: restake_validator.to_string(),
            amount: Coin {
                denom: INJ.to_string(),
                amount: restake_amount,
            },
        });
        Ok(res)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetStakerInfo {} => to_json_binary(&query::get_staker_info(deps)?),
        QueryMsg::TokenInfo {} => to_json_binary(&query_token_info(deps)?),
        QueryMsg::MarketingInfo {} => to_json_binary(&query_marketing_info(deps)?),
        QueryMsg::Balance { address } => to_json_binary(&query_balance(deps, address)?),
        QueryMsg::GetValidators {} => to_json_binary(&query::get_validators(deps, env)?),
        QueryMsg::GetTotalSupply {} => to_json_binary(&query::get_total_supply(deps)?),
        QueryMsg::GetTotalStaked {} => {
            to_json_binary(&query::get_total_staked(deps, env.contract.address)?)
        }
        QueryMsg::GetTotalRewards {} => {
            to_json_binary(&query::get_total_rewards(deps, env.contract.address)?)
        }
        QueryMsg::IsAgent { agent } => to_json_binary(&query::is_agent(deps, agent)?),
        QueryMsg::IsOwner { addr } => to_json_binary(&query::is_owner(deps, addr)?),
        QueryMsg::IsWhitelisted { user } => {
            to_json_binary(&query::is_user_whitelisted(deps, user)?)
        }
        QueryMsg::IsBlacklisted { user } => {
            to_json_binary(&query::is_user_blacklisted(deps, user)?)
        }
        QueryMsg::GetCurrentUserStatus { user } => {
            to_json_binary(&query::get_current_user_status(deps, user)?)
        }
        QueryMsg::GetSharePrice {} => {
            to_json_binary(&query::get_share_price(deps, env.contract.address))
        }
        QueryMsg::GetTotalAssets {} => {
            to_json_binary(&query::get_total_assets(deps, env.contract.address)?)
        }
        QueryMsg::GetClaimableAssets { user } => {
            to_json_binary(&query::get_claimable_assets(deps, user)?)
        }
        QueryMsg::GetMaxWithdraw { user } => {
            to_json_binary(&query::get_max_withdraw(deps, env.contract.address, user)?)
        }
        QueryMsg::GetClaimableAmount { user } => {
            to_json_binary(&query::get_claimable_amount(deps, env, user)?)
        }
        QueryMsg::GetAllocations { user } => to_json_binary(&query::get_allocations(deps, user)?),
        QueryMsg::GetTotalAllocated { user } => {
            to_json_binary(&query::get_total_allocated(deps, user)?)
        }
        QueryMsg::GetDistributionAmounts {
            distributor,
            recipient,
        } => to_json_binary(&query::get_distribution_amounts(
            deps,
            env.contract.address,
            distributor,
            recipient,
        )?),
    }
}

pub mod query {
    use cosmwasm_std::Order;
    use cw_controllers::ClaimsResponse;

    use crate::msg::{
        GetAllocationsResponse, GetClaimableAmountResponse, GetMaxWithdrawResponse,
        GetTotalAllocatedResponse, GetTotalAssetsResponse, GetTotalRewardsResponse,
        GetTotalStakedResponse, GetTotalSupplyResponse, GetValidatorResponse,
    };

    use super::*;
    use crate::msg::{
        GetCurrentUserStatusResponse, GetIsAgentResponse, GetIsBlacklistedResponse,
        GetIsOwnerResponse, GetIsWhitelistedResponse,
    };
    use crate::state::{Allocation, ValidatorInfo, VALIDATORS};
    use cosmwasm_std::Addr;

    pub fn get_staker_info(deps: Deps) -> StdResult<GetStakerInfoResponse> {
        let staker_info = STAKER_INFO.load(deps.storage)?;
        Ok(GetStakerInfoResponse {
            owner: OWNER.load(deps.storage)?,
            default_validator: DEFAULT_VALIDATOR.load(deps.storage)?,
            treasury: staker_info.treasury,
            fee: staker_info.fee,
            distribution_fee: staker_info.distribution_fee,
            min_deposit: staker_info.min_deposit.into(),
            is_paused: IS_PAUSED.load(deps.storage)?,
        })
    }

    /// Returns all available validators and their info.
    pub fn get_validators(deps: Deps, env: Env) -> StdResult<GetValidatorResponse> {
        let contract = env.contract.address;

        let validators = VALIDATORS
            .range(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<(_, Validator)>>>()
            .map(|validators| {
                validators
                    .into_iter()
                    .map(|(validator_addr, validator)| {
                        let mut total_staked = Uint128::zero();
                        if let Some(delegation) = deps
                            .querier
                            .query_delegation(contract.clone(), validator_addr.clone())
                            .unwrap()
                        {
                            total_staked = delegation.amount.amount;
                        }
                        ValidatorInfo {
                            addr: validator_addr,
                            total_staked,
                            state: validator.state,
                        }
                    })
                    .collect()
            });

        Ok(GetValidatorResponse {
            validators: validators?,
        })
    }

    /// Returns the total supply of TruINJ.
    pub fn get_total_supply(deps: Deps) -> StdResult<GetTotalSupplyResponse> {
        Ok(GetTotalSupplyResponse {
            total_supply: TOKEN_INFO.load(deps.storage)?.total_supply,
        })
    }

    // Returns the total staked across all validators.
    pub fn get_total_staked(
        deps: Deps,
        contract_address: Addr,
    ) -> StdResult<GetTotalStakedResponse> {
        let (total_staked, _) = get_total_staked_and_rewards(deps, contract_address).unwrap();
        Ok(GetTotalStakedResponse {
            total_staked: total_staked.into(),
        })
    }

    // Returns the total rewards across all validators.
    pub fn get_total_rewards(
        deps: Deps,
        contract_address: Addr,
    ) -> StdResult<GetTotalRewardsResponse> {
        let (_, total_rewards) = get_total_staked_and_rewards(deps, contract_address).unwrap();
        Ok(GetTotalRewardsResponse {
            total_rewards: total_rewards.into(),
        })
    }

    /// Returns the amount of INJ held by the contract.
    pub fn get_total_assets(
        deps: Deps,
        contract_address: Addr,
    ) -> StdResult<GetTotalAssetsResponse> {
        let total_assets = deps
            .querier
            .query_balance(contract_address, INJ)
            .unwrap()
            .amount;

        Ok(GetTotalAssetsResponse { total_assets })
    }

    /// Returns the maximum amount of assets that can be withdrawn by the user.
    pub fn get_max_withdraw(
        deps: Deps,
        contract_address: Addr,
        user: Addr,
    ) -> StdResult<GetMaxWithdrawResponse> {
        let shares = query_balance(deps, user.to_string())?.balance.u128();
        let share_price = get_share_price(deps, contract_address);
        let assets =
            convert_to_assets(shares, share_price.numerator, share_price.denominator, true)
                .unwrap();

        Ok(GetMaxWithdrawResponse {
            max_withdraw: assets.into(),
        })
    }

    /// Returns the list of outstanding claims for a user.
    pub fn get_claimable_assets(deps: Deps, user: Addr) -> StdResult<ClaimsResponse> {
        let claim_response = CLAIMS.query_claims(deps, &user)?;

        Ok(claim_response)
    }

    pub fn is_agent(deps: Deps, agent: Addr) -> StdResult<GetIsAgentResponse> {
        Ok(GetIsAgentResponse {
            is_agent: whitelist::is_agent(deps, agent).unwrap(),
        })
    }

    pub fn is_owner(deps: Deps, addr: Addr) -> StdResult<GetIsOwnerResponse> {
        let owner = OWNER.load(deps.storage)?;
        Ok(GetIsOwnerResponse {
            is_owner: addr == owner,
        })
    }

    pub fn is_user_whitelisted(deps: Deps, user: Addr) -> StdResult<GetIsWhitelistedResponse> {
        Ok(GetIsWhitelistedResponse {
            is_whitelisted: whitelist::is_user_whitelisted(deps, user),
        })
    }

    pub fn is_user_blacklisted(deps: Deps, user: Addr) -> StdResult<GetIsBlacklistedResponse> {
        Ok(GetIsBlacklistedResponse {
            is_blacklisted: whitelist::is_user_blacklisted(deps, user),
        })
    }

    pub fn get_current_user_status(
        deps: Deps,
        user: Addr,
    ) -> StdResult<GetCurrentUserStatusResponse> {
        Ok(GetCurrentUserStatusResponse {
            user_status: whitelist::get_current_user_status(deps, &user).unwrap(),
        })
    }

    pub fn get_claimable_amount(
        deps: Deps,
        env: Env,
        sender: Addr,
    ) -> StdResult<GetClaimableAmountResponse> {
        let block = env.block;
        let claimable_amount = CLAIMS.query_claims(deps, &sender)?.claims.iter().fold(
            Uint128::zero(),
            |acc, claim| {
                if claim.release_at.is_expired(&block) {
                    acc + claim.amount
                } else {
                    acc
                }
            },
        );

        Ok(GetClaimableAmountResponse { claimable_amount })
    }

    /// Returns the current TruINJ share price in INJ.
    pub fn get_share_price(deps: Deps, contract_address: Addr) -> GetSharePriceResponse {
        let (total_staked, total_rewards) =
            get_total_staked_and_rewards(deps, contract_address).unwrap();
        let total_assets = CONTRACT_REWARDS.load(deps.storage).unwrap().u128();
        let shares_supply = TOKEN_INFO.load(deps.storage).unwrap().total_supply.u128();
        let fee = STAKER_INFO.load(deps.storage).unwrap().fee;

        let (share_price_num, share_price_denom) = internal_share_price(
            total_staked,
            total_assets,
            total_rewards,
            shares_supply,
            fee,
        );

        GetSharePriceResponse {
            numerator: share_price_num,
            denominator: share_price_denom,
        }
    }

    /// Returns all allocations for a given user.
    pub fn get_allocations(deps: Deps, allocator: Addr) -> StdResult<GetAllocationsResponse> {
        let allocations: Vec<Allocation> = allocations()
            .idx
            .allocator
            .prefix(allocator)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| item.map(|(_, allocation)| allocation))
            .collect::<StdResult<Vec<_>>>()?;

        Ok(GetAllocationsResponse { allocations })
    }

    // Function to view the total amount of INJ allocated by a user and their average allocation share price.
    pub fn get_total_allocated(deps: Deps, user: Addr) -> StdResult<GetTotalAllocatedResponse> {
        let allocations = allocations()
            .idx
            .allocator
            .prefix(user)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| item.map(|(_, allocation)| allocation))
            .collect::<StdResult<Vec<_>>>()?;

        let total_allocation = allocations
            .into_iter()
            .reduce(|acc, allocation| {
                calculate_updated_allocation(
                    &acc,
                    allocation.inj_amount,
                    allocation.share_price_num,
                    allocation.share_price_denom,
                )
                .unwrap()
            })
            .unwrap_or_default();

        Ok(GetTotalAllocatedResponse {
            total_allocated_amount: total_allocation.inj_amount,
            total_allocated_share_price_num: total_allocation.share_price_num,
            total_allocated_share_price_denom: total_allocation.share_price_denom,
        })
    }

    /// Returns the amounts of INJ and TruINJ that a user needs to hold in order to distribute to a recipient or all recipients.
    /// Also returns the distribution fee, in TruINJ, that will be paid to the treasury.
    pub fn get_distribution_amounts(
        deps: Deps,
        contract_address: Addr,
        distributor: Addr,
        recipient: Option<Addr>,
    ) -> StdResult<GetDistributionAmountsResponse> {
        let share_price = get_share_price(deps, contract_address);
        let fee = STAKER_INFO.load(deps.storage)?.distribution_fee;

        let (total_inj_amount, total_truinj_amount, total_fees) = if let Some(recipient) = recipient
        {
            let allocation = match allocations().may_load(deps.storage, (distributor, recipient))? {
                Some(allocation) => allocation,
                None => {
                    return Ok(GetDistributionAmountsResponse {
                        inj_amount: Uint128::zero(),
                        truinj_amount: Uint128::zero(),
                        distribution_fee: Uint128::zero(),
                    });
                }
            };

            let (inj_amount, truinj_amount, fees) = calculate_distribution_amounts(
                &allocation,
                share_price.numerator,
                share_price.denominator,
                fee,
            )
            .unwrap();

            (inj_amount, truinj_amount, fees)
        } else {
            let (total_inj_amount, total_truinj_amount, total_fees) = allocations()
                .idx
                .allocator
                .prefix(distributor)
                .range(deps.storage, None, None, Order::Ascending)
                .fold((0, 0, 0), |acc, item| {
                    let (_, allocation) = item.unwrap();
                    let (inj_amount, truinj_amount, fees) = calculate_distribution_amounts(
                        &allocation,
                        share_price.numerator,
                        share_price.denominator,
                        fee,
                    )
                    .unwrap();
                    (acc.0 + inj_amount, acc.1 + truinj_amount, acc.2 + fees)
                });

            (total_inj_amount, total_truinj_amount, total_fees)
        };

        Ok(GetDistributionAmountsResponse {
            inj_amount: total_inj_amount.into(),
            truinj_amount: total_truinj_amount.into(),
            distribution_fee: total_fees.into(),
        })
    }
}

/// Checks that the caller is the owner of the contract.
fn check_owner(deps: Deps, user_address: Addr) -> Result<(), ContractError> {
    let owner = OWNER.load(deps.storage)?;
    ensure!(user_address == owner, ContractError::OnlyOwner);
    Ok(())
}

fn check_not_paused(deps: Deps) -> Result<(), ContractError> {
    ensure!(
        !IS_PAUSED.load(deps.storage)?,
        ContractError::ContractPaused
    );
    Ok(())
}

// Checks that the chosen validator exists and is enabled.
fn check_validator(deps: Deps, validator_addr: Addr) -> Result<(), ContractError> {
    let validator = VALIDATORS
        .may_load(deps.storage, &validator_addr)?
        .ok_or(ContractError::ValidatorDoesNotExist)?;
    ensure!(
        validator.state == ValidatorState::ENABLED,
        ContractError::ValidatorNotEnabled
    );
    Ok(())
}

// Function to get the total staked and reward amounts across all validators
fn get_total_staked_and_rewards(
    deps: Deps,
    contract_address: Addr,
) -> Result<(u128, u128), ContractError> {
    let mut total_staked = 0u128;
    let mut total_rewards = 0u128;

    for validator in VALIDATORS.range(deps.storage, None, None, cosmwasm_std::Order::Ascending) {
        let (validator_addr, _) = validator?;

        if let Some(delegation) = deps
            .querier
            .query_delegation(contract_address.clone(), validator_addr)?
        {
            total_staked += delegation.amount.amount.u128();

            if let Some(reward) = delegation
                .accumulated_rewards
                .iter()
                .find(|coin| coin.denom == INJ)
            {
                total_rewards += reward.amount.u128();
            }
        }
    }

    Ok((total_staked, total_rewards))
}

// Stakes the attached INJ to the specified validator.
fn internal_stake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    validator_addr: Addr,
) -> Result<Response, ContractError> {
    check_validator(deps.as_ref(), validator_addr.clone())?;

    let staker_info = STAKER_INFO.load(deps.storage)?;

    // check deposited INJ amount is above the minimum
    let stake_amount = cw_utils::must_pay(&info, INJ)?;
    ensure!(
        stake_amount.u128() >= staker_info.min_deposit,
        ContractError::DepositBelowMinDeposit
    );

    let staker_address = env.contract.address.clone();
    let user = info.sender.to_string();

    // fetch data needed to compute the share price
    let (total_staked, total_rewards) =
        get_total_staked_and_rewards(deps.as_ref(), staker_address.clone())?;

    let contract_rewards: Uint128 = CONTRACT_REWARDS.load(deps.storage)?;

    let shares_supply = TOKEN_INFO.load(deps.storage)?.total_supply;
    let fee = staker_info.fee;

    let (share_price_num, share_price_denom) = internal_share_price(
        total_staked,
        contract_rewards.u128(),
        total_rewards,
        shares_supply.u128(),
        fee,
    );

    let validator_total_rewards = deps
        .querier
        .query_delegation(staker_address, validator_addr.clone())?
        .and_then(|d| {
            d.accumulated_rewards
                .iter()
                .find(|coin| coin.denom == INJ)
                .cloned()
        })
        .map(|reward| reward.amount.u128())
        .unwrap_or(0);

    CONTRACT_REWARDS.save(deps.storage, &validator_total_rewards.into())?;

    // mint fees to the treasury for the liquid rewards on the validator
    let treasury_shares_minted = mint_treasury_fees(
        &mut deps,
        &env,
        validator_total_rewards,
        fee,
        staker_info.treasury.clone(),
        share_price_num,
        share_price_denom,
    )?;

    // calculate the shares to mint to the user
    let user_shares_increase = convert_to_shares(stake_amount, share_price_num, share_price_denom)?;

    // mint shares to the user
    let contract_addr = env.contract.address.clone();
    execute_mint(
        deps.branch(),
        env,
        MessageInfo {
            sender: contract_addr,
            funds: vec![],
        },
        user.clone(),
        user_shares_increase,
    )?;
    let user_balance = query_balance(deps.as_ref(), user)?.balance;

    // sweep contract rewards
    let new_stake_amount = stake_amount + contract_rewards;

    // delegate to the validator
    let delegate_msg = StakingMsg::Delegate {
        validator: validator_addr.to_string(),
        amount: Coin {
            denom: INJ.to_string(),
            amount: new_stake_amount,
        },
    };

    let new_shares_total_supply = shares_supply + user_shares_increase + treasury_shares_minted;

    Ok(Response::new().add_message(delegate_msg).add_event(
        Event::new("deposited")
            .add_attribute("user", info.sender)
            .add_attribute("validator_addr", validator_addr)
            .add_attribute("amount", stake_amount)
            .add_attribute("contract_rewards", contract_rewards)
            .add_attribute("user_shares_minted", user_shares_increase)
            .add_attribute("treasury_shares_minted", treasury_shares_minted)
            .add_attribute(
                "treasury_balance",
                query_balance(deps.as_ref(), staker_info.treasury.into_string())?.balance,
            )
            .add_attribute(
                "total_staked",
                Uint128::from(total_staked) + new_stake_amount,
            )
            .add_attribute("total_supply", new_shares_total_supply)
            .add_attribute("share_price_num", share_price_num)
            .add_attribute("share_price_denom", share_price_denom)
            .add_attribute("user_balance", user_balance),
    ))
}

// Unstakes a given amount of INJ from a specific validator.
fn internal_unstake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    validator_addr: Addr,
    assets: u128,
) -> Result<Response, ContractError> {
    let user_addr = info.sender.clone();
    let contract_addr = env.contract.address.clone();

    // check that the amount of assets to unstake is greater than 0
    ensure!(assets > 0, ContractError::UnstakeAmountTooLow);

    // calculate the current share price
    let (total_staked, total_rewards) =
        get_total_staked_and_rewards(deps.as_ref(), contract_addr.clone())?;
    let contract_rewards = CONTRACT_REWARDS.load(deps.storage)?;
    let shares_supply = TOKEN_INFO.load(deps.storage).unwrap().total_supply.u128();
    let staker_info = STAKER_INFO.load(deps.storage).unwrap();
    let fee = staker_info.fee;

    let (share_price_num, share_price_denom) = internal_share_price(
        total_staked,
        contract_rewards.u128(),
        total_rewards,
        shares_supply,
        fee,
    );

    // get max withdrawable amount
    let shares_balance = query_balance(deps.as_ref(), user_addr.to_string())?
        .balance
        .u128();
    let max_withdraw = convert_to_assets(shares_balance, share_price_num, share_price_denom, true)?;

    // check the user has enough shares
    ensure!(
        assets <= max_withdraw,
        ContractError::InsufficientTruINJBalance
    );

    // if the remaining asset balance is below the min deposit the entire balance is withdrawn and all shares are burnt.
    // otherwise, we calculate the shares to burn based on the amount of INJ to unstake and the share price.
    let min_deposit = staker_info.min_deposit;
    let (assets_to_unstake, shares_to_burn) = if max_withdraw - assets < min_deposit {
        (max_withdraw, shares_balance)
    } else {
        // calculate the user shares to burn
        let shares = convert_to_shares(assets.into(), share_price_num, share_price_denom)?.u128();
        (assets, shares)
    };

    // check that the amount of shares to burn is greater than 0
    ensure!(shares_to_burn > 0, ContractError::SharesAmountTooLow);

    let (validator_total_staked, validator_total_rewards) = deps
        .querier
        .query_delegation(contract_addr, validator_addr.clone())?
        .map(|d| {
            let total_staked = d.amount.amount.u128();
            let total_rewards = d
                .accumulated_rewards
                .iter()
                .find(|coin| coin.denom == INJ)
                .map(|reward| reward.amount.u128())
                .unwrap_or(0);
            (total_staked, total_rewards)
        })
        .unwrap_or((0, 0));

    // Assert the validator has enough funds to unstake.
    ensure!(
        assets_to_unstake <= validator_total_staked + validator_total_rewards + 1,
        ContractError::InsufficientValidatorFunds
    );

    // A user may unstake more than is currently staked on a validator, if the total amount to unstake is less than or
    // equal to the total staked + rewards on the validator. In this rare case, we will only unstake the total staked,
    // but account for the additional unstake amount from the rewards. The reasoning behind this, is so that if there is
    // a sole user, they should be able to withdraw their max_withdraw amount in one transaction.
    let mut actual_amount_to_unstake = assets_to_unstake;
    let mut excess_unstaked_amount = 0;
    if actual_amount_to_unstake > validator_total_staked {
        excess_unstaked_amount = actual_amount_to_unstake - validator_total_staked;
        actual_amount_to_unstake = validator_total_staked;
    }

    // when unstaking, all accrued rewards are moved into the validator.
    // We discount the excess unstaked amount from the rewards because it belongs to the user performing this unstake operation.
    CONTRACT_REWARDS.update(deps.storage, |mut rewards| -> Result<_, ContractError> {
        rewards += Uint128::from(validator_total_rewards - excess_unstaked_amount);
        Ok(rewards)
    })?;

    // add unbond request to the claims list
    CLAIMS.create_claim(
        deps.storage,
        &user_addr,
        assets_to_unstake.into(),
        UNBONDING_PERIOD.after(&env.block),
    )?;

    // mint fees to the treasury for the liquid rewards on the validator
    let treasury_shares_minted = mint_treasury_fees(
        &mut deps,
        &env,
        validator_total_rewards,
        fee,
        staker_info.treasury.clone(),
        share_price_num,
        share_price_denom,
    )?;

    // burn the user shares
    execute_burn(deps.branch(), env, info, shares_to_burn.into())?;
    let user_shares_balance = query_balance(deps.as_ref(), user_addr.to_string())?.balance;

    let new_total_staked = total_staked - actual_amount_to_unstake;
    let new_shares_supply = shares_supply + treasury_shares_minted.u128() - shares_to_burn;

    // send undelegate message and emit event
    let res = Response::new()
        .add_message(StakingMsg::Undelegate {
            validator: validator_addr.to_string(),
            amount: Coin {
                denom: INJ.to_string(),
                amount: actual_amount_to_unstake.into(),
            },
        })
        .add_event(
            Event::new("unstaked")
                .add_attribute("user", user_addr)
                .add_attribute("amount", assets_to_unstake.to_string())
                .add_attribute("validator_addr", validator_addr)
                .add_attribute("user_balance", user_shares_balance)
                .add_attribute("user_shares_burned", shares_to_burn.to_string())
                .add_attribute("treasury_shares_minted", treasury_shares_minted.to_string())
                .add_attribute(
                    "treasury_balance",
                    query_balance(deps.as_ref(), staker_info.treasury.into_string())?.balance,
                )
                .add_attribute("total_staked", new_total_staked.to_string())
                .add_attribute("total_supply", new_shares_supply.to_string()),
        );

    Ok(res)
}

/// Converts an amount of INJ tokens to the equivalent TruINJ amount.
fn convert_to_shares(
    inj_amount: Uint128,
    share_price_num: Uint256,
    share_price_denom: Uint256,
) -> Result<Uint128, ContractError> {
    let mul: Uint512 = Uint512::from(share_price_denom)
        * Uint512::from(inj_amount)
        * Uint512::from(SHARE_PRICE_SCALING_FACTOR);
    let div = Uint256::try_from(mul.checked_div(share_price_num.into())?)?;
    let shares = Uint128::try_from(div)?;
    Ok(shares)
}

/// Converts an amount of TruINJ shares to the equivalent INJ amount, with the desired rounding.
fn convert_to_assets(
    shares: u128,
    share_price_num: Uint256,
    share_price_denom: Uint256,
    rounding_up: bool,
) -> Result<u128, ContractError> {
    let x = Uint512::from(shares);
    let y = Uint512::from(share_price_num);
    let denominator = Uint512::from(share_price_denom)
        .checked_mul(Uint512::from(SHARE_PRICE_SCALING_FACTOR))
        .unwrap();

    let mut assets = x * y / denominator;
    let remainder = (x * y) % denominator;

    if rounding_up && !remainder.is_zero() {
        assets += Uint512::one();
    }

    let result = Uint128::try_from(assets)?;
    Ok(result.u128())
}

/// Calculates the share price using the provided parameters.
fn internal_share_price(
    total_staked: u128,
    total_assets: u128,
    total_rewards: u128,
    shares_supply: u128,
    fee: u16,
) -> (Uint256, Uint256) {
    if shares_supply == 0 {
        return (Uint256::from(SHARE_PRICE_SCALING_FACTOR), Uint256::one());
    };
    let total_capital = Uint256::from(total_staked + total_assets) * Uint256::from(FEE_PRECISION)
        + Uint256::from(total_rewards) * Uint256::from(FEE_PRECISION - fee);

    let price_num = total_capital * Uint256::from(SHARE_PRICE_SCALING_FACTOR);
    let price_denom = Uint256::from(shares_supply) * Uint256::from(FEE_PRECISION);

    (price_num, price_denom)
}

fn internal_distribute(
    mut deps: DepsMut,
    env: Env,
    distributor: &Addr,
    recipient: &Addr,
    allocation: Allocation,
    in_inj: bool,
    global_price_num: Uint256,
    global_price_denom: Uint256,
    attached_inj_amount: u128,
) -> Result<Option<DistributionInfo>, ContractError> {
    // if the share price of the allocation is the same as the global share price, no distribution is needed
    if allocation.share_price_num / allocation.share_price_denom
        == global_price_num / global_price_denom
    {
        return Ok(None);
    }

    let staker_info = STAKER_INFO.load(deps.storage)?;
    let treasury = staker_info.treasury;
    let dist_fee = staker_info.distribution_fee;

    let (assets_to_distribute, shares_to_distribute, fees) =
        calculate_distribution_amounts(&allocation, global_price_num, global_price_denom, dist_fee)
            .unwrap();

    if in_inj {
        ensure!(
            attached_inj_amount >= assets_to_distribute,
            ContractError::InsufficientInjAttached
        );
        ensure!(
            query_balance(deps.as_ref(), distributor.to_string())?
                .balance
                .u128()
                >= fees,
            ContractError::InsufficientTruINJBalance
        );
    } else {
        // check that the distributor has enough TruINJ to distribute and pay the fees
        ensure!(
            query_balance(deps.as_ref(), distributor.to_string())?
                .balance
                .u128()
                >= shares_to_distribute + fees,
            ContractError::InsufficientTruINJBalance
        );

        // transfer the rewards in TruINJ to the recipient
        execute_transfer(
            deps.branch(),
            env.clone(),
            MessageInfo {
                sender: distributor.clone(),
                funds: vec![],
            },
            recipient.to_string(),
            Uint128::from(shares_to_distribute),
        )?;
    };

    // transfer fees to the treasury
    if fees > 0 {
        execute_transfer(
            deps.branch(),
            env,
            MessageInfo {
                sender: distributor.clone(),
                funds: vec![],
            },
            treasury.to_string(),
            Uint128::from(fees),
        )?;
    }

    // determine the amount of INJ to refund to the distributor
    let refund_amount = if in_inj {
        attached_inj_amount - assets_to_distribute
    } else {
        attached_inj_amount
    };

    let distribution_info = DistributionInfo {
        user: distributor.clone(),
        recipient: recipient.clone(),
        user_balance: query_balance(deps.as_ref(), distributor.to_string())?
            .balance
            .u128(),
        recipient_balance: query_balance(deps.as_ref(), recipient.to_string())?
            .balance
            .u128(),
        treasury_balance: query_balance(deps.as_ref(), treasury.to_string())?
            .balance
            .u128(),
        shares: shares_to_distribute,
        inj_amount: assets_to_distribute,
        in_inj,
        share_price_num: global_price_num,
        share_price_denom: global_price_denom,
        fees,
        refund_amount,
    };

    Ok(Some(distribution_info))
}

/// Mints fees to the treasury for the amount of staking rewards provided.
fn mint_treasury_fees(
    deps: &mut DepsMut<'_>,
    env: &Env,
    rewards: u128,
    fee: u16,
    treasury_addr: Addr,
    share_price_num: Uint256,
    share_price_denom: Uint256,
) -> Result<Uint128, ContractError> {
    // calculate the fees in TruINJ to mint to the treasury
    let fees = rewards * fee as u128 / FEE_PRECISION as u128;
    let treasury_shares_increase =
        convert_to_shares(Uint128::from(fees), share_price_num, share_price_denom)?;

    execute_mint(
        deps.branch(),
        env.clone(),
        MessageInfo {
            sender: env.contract.address.clone(),
            funds: vec![],
        },
        treasury_addr.to_string(),
        treasury_shares_increase,
    )?;
    Ok(treasury_shares_increase)
}

/// Calculates the updated allocation values.
fn calculate_updated_allocation(
    existing: &Allocation,
    amount: Uint128,
    global_share_price_num: Uint256,
    global_share_price_denom: Uint256,
) -> Result<Allocation, ContractError> {
    let mul_lhs = Uint512::from(existing.inj_amount)
        * Uint512::from(SHARE_PRICE_SCALING_FACTOR)
        * Uint512::from(existing.share_price_denom);
    let denominator_lhs = Uint256::try_from(mul_lhs.checked_div(existing.share_price_num.into())?)?;

    let mul_rhs = Uint512::from(amount)
        * Uint512::from(SHARE_PRICE_SCALING_FACTOR)
        * Uint512::from(global_share_price_denom);
    let denominator_rhs = Uint256::try_from(mul_rhs.checked_div(global_share_price_num.into())?)?;

    let share_price_denom = denominator_lhs + denominator_rhs;

    let share_price_num = (Uint256::from(existing.inj_amount) + Uint256::from(amount))
        * Uint256::from(SHARE_PRICE_SCALING_FACTOR);

    Ok(Allocation {
        allocator: existing.allocator.clone(),
        recipient: existing.recipient.clone(),
        inj_amount: existing.inj_amount + amount,
        share_price_num,
        share_price_denom,
    })
}

#[cfg(any(test, feature = "test"))]
pub fn test_allocate(
    deps: DepsMut,
    env: Env,
    distributor: Addr,
    recipient: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    use query::get_share_price;

    let share_price_response = get_share_price(deps.as_ref(), env.contract.address);
    allocations().save(
        deps.storage,
        (distributor.clone(), recipient.clone()),
        &Allocation {
            allocator: distributor,
            recipient,
            inj_amount: amount,
            share_price_num: share_price_response.numerator,
            share_price_denom: share_price_response.denominator,
        },
    )?;

    Ok(Response::new().add_event(Event::new("allocated")))
}

/// For a given allocation returns a tuple containing the amount of INJ and TruINJ required for the distribution,
/// as well as the fees in TruINJ that will be paid to the treasury.
fn calculate_distribution_amounts(
    allocation: &Allocation,
    global_price_num: Uint256,
    global_price_denom: Uint256,
    distribution_fee: u16,
) -> Result<(u128, u128, u128), ContractError> {
    let alloc_amount = Uint512::from(allocation.inj_amount);
    let alloc_share_price_num = Uint512::from(allocation.share_price_num);
    let alloc_share_price_denom =
        Uint512::from(allocation.share_price_denom) * Uint512::from(SHARE_PRICE_SCALING_FACTOR);

    let global_share_price_num = Uint512::from(global_price_num);
    let global_share_price_denom =
        Uint512::from(global_price_denom) * Uint512::from(SHARE_PRICE_SCALING_FACTOR);

    let shares_before_fees = Uint128::try_from(
        alloc_amount * alloc_share_price_denom / alloc_share_price_num
            - alloc_amount * global_share_price_denom / global_share_price_num,
    )?
    .u128();

    // calculate the distribution fees in TruINJ to mint to the treasury
    let fees: u128 = shares_before_fees * distribution_fee as u128 / FEE_PRECISION as u128;

    // calculate the net shares to distribute
    let shares_to_distribute = shares_before_fees - fees;

    // convert the shares to INJ
    let assets_to_distribute = convert_to_assets(
        shares_to_distribute,
        global_price_num,
        global_price_denom,
        false,
    )?;

    Ok((assets_to_distribute, shares_to_distribute, fees))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::state::{UserStatus, IS_PAUSED, WHITELIST_USERS};
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{
        coins, from_json, Addr, ConversionOverflowError, DivideByZeroError, Uint128,
    };
    use cw20::BalanceResponse;
    use cw_multi_test::{App, ContractWrapper, Executor, IntoBech32};

    #[test]
    fn test_initialization() {
        let mut deps = mock_dependencies();

        let owner: Addr = "owner".into_bech32();
        let default_validator: Addr = "my-validator".into_bech32();
        let treasury: Addr = "treasury".into_bech32();

        let res = instantiate(
            deps.as_mut(),
            mock_env(),
            message_info(&owner, &coins(1000, INJ)),
            InstantiateMsg {
                default_validator: default_validator.clone(),
                treasury: treasury.clone(),
            },
        )
        .unwrap();

        // verify the response
        assert_eq!(res.messages.len(), 0);
        assert_eq!(res.attributes.len(), 0);
        assert_eq!(res.events.len(), 1);

        // verify the instantiate event was emitted
        assert_eq!(
            res.events[0],
            Event::new("instantiated")
                .add_attribute("owner", owner.clone())
                .add_attribute("default_validator", default_validator.clone())
                .add_attribute("treasury", treasury.clone())
                .add_attribute("token_name", "TruINJ")
        );

        // query the staker info and check its properties
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetStakerInfo {}).unwrap();
        let value: GetStakerInfoResponse = from_json(&res).unwrap();
        assert_eq!(value.default_validator, default_validator);
        assert_eq!(value.treasury, treasury);
        assert_eq!(value.owner, owner);
    }

    #[test]
    fn instantiation_mints_no_tokens_to_owner() {
        let mut app = App::default();
        let owner: Addr = "owner".into_bech32();
        let default_validator: Addr = "my-validator".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let user = app.api().addr_make("user");

        let addr = app
            .instantiate_contract(
                code_id,
                owner,
                &InstantiateMsg {
                    default_validator,
                    treasury,
                },
                &[],
                "Contract",
                None,
            )
            .unwrap();

        let resp: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                addr,
                &QueryMsg::Balance {
                    address: user.to_string(),
                },
            )
            .unwrap();

        assert_eq!(
            resp,
            BalanceResponse {
                balance: Uint128::zero()
            }
        );
    }

    #[test]
    fn test_check_whitelisted_with_whitelisted_user() {
        let mut deps = mock_dependencies();

        // mock a whitelisted user
        let user = "user".into_bech32();
        let _ = WHITELIST_USERS.save(&mut deps.storage, &user, &UserStatus::Whitelisted);

        // verify that Ok() is returned
        assert!(whitelist::check_whitelisted(deps.as_ref(), user).is_ok())
    }

    #[test]
    fn test_check_whitelisted_with_non_whitelisted_user() {
        let deps = mock_dependencies();

        // a non whitelisted user
        let user = "user".into_bech32();

        // verify that the expected error is returned
        let error = whitelist::check_whitelisted(deps.as_ref(), user);
        assert!(error.is_err());
        assert_eq!(error, Err(ContractError::UserNotWhitelisted));
    }

    #[test]
    fn test_check_not_paused_when_not_paused() {
        // mock contract is not paused
        let mut deps = mock_dependencies();
        IS_PAUSED.save(&mut deps.storage, &false).unwrap();

        // verify that check_not_paused returns Ok(())
        let result = check_not_paused(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_not_paused_when_paused() {
        // mock contract is paused
        let mut deps = mock_dependencies();
        IS_PAUSED.save(&mut deps.storage, &true).unwrap();

        // verify that check_not_paused returns the expected error
        let result = check_not_paused(deps.as_ref());
        assert_eq!(result.err().unwrap(), ContractError::ContractPaused);
    }

    #[test]
    fn test_share_price_with_zero_shares_supply() {
        let total_staked: u128 = 0;
        let total_assets: u128 = 0;
        let total_rewards: u128 = 0;
        let shares_supply: u128 = 0;
        let fee: u16 = 0;

        let (num, denom) = internal_share_price(
            total_staked,
            total_assets,
            total_rewards,
            shares_supply,
            fee,
        );

        assert_eq!(num, Uint256::from(SHARE_PRICE_SCALING_FACTOR));
        assert_eq!(denom, Uint256::from(1u64));
    }

    #[test]
    fn test_share_price_with_total_staked_matching_share_supply() {
        let total_staked: u128 = 100 * ONE_INJ;
        let total_assets: u128 = 0;
        let total_rewards: u128 = 0;
        let shares_supply: u128 = 100 * ONE_INJ;
        let fee: u16 = 0;

        let (num, denom) = internal_share_price(
            total_staked,
            total_assets,
            total_rewards,
            shares_supply,
            fee,
        );

        // verify the share price numerator and denominator
        let expected_num = Uint256::from(total_staked)
            * Uint256::from(FEE_PRECISION as u128)
            * Uint256::from(SHARE_PRICE_SCALING_FACTOR);
        let expected_denom = Uint256::from(shares_supply) * Uint256::from(FEE_PRECISION as u128);
        assert_eq!(num, expected_num);
        assert_eq!(denom, expected_denom);

        // verify that the share price is 1.0
        let share_price = num / denom;
        assert_eq!(share_price, Uint256::from(SHARE_PRICE_SCALING_FACTOR));
    }

    #[test]
    fn test_share_price_with_total_staked_and_total_assets() {
        let total_staked: u128 = 226 * ONE_INJ;
        let total_assets: u128 = 20 * ONE_INJ;
        let total_rewards: u128 = 0;
        let shares_supply: u128 = 200 * ONE_INJ;
        let fee: u16 = 0;

        let (num, denom) = internal_share_price(
            total_staked,
            total_assets,
            total_rewards,
            shares_supply,
            fee,
        );

        let expected_num = Uint256::from(total_staked + total_assets)
            * Uint256::from(FEE_PRECISION)
            * Uint256::from(SHARE_PRICE_SCALING_FACTOR);
        let expected_denom = Uint256::from(shares_supply) * Uint256::from(FEE_PRECISION);
        assert_eq!(num, expected_num);
        assert_eq!(denom, expected_denom);

        // verify that the share price is 1.23 INJ
        let share_price = num / denom;
        assert_eq!(share_price, Uint256::from(1230000000000000000u64));
    }

    #[test]
    fn test_share_price_with_fees() {
        let total_staked: u128 = 326 * ONE_INJ;
        let total_assets: u128 = 20 * ONE_INJ;
        let total_rewards: u128 = 100 * ONE_INJ;
        let shares_supply: u128 = 200 * ONE_INJ;
        let fee: u16 = 500; // 5%

        let (num, denom) = internal_share_price(
            total_staked,
            total_assets,
            total_rewards,
            shares_supply,
            fee,
        );

        // verify the share price numerator and denominator
        let expected_fees = 5 * ONE_INJ;
        let expected_num =
            Uint256::from(total_staked + total_assets + total_rewards - expected_fees)
                * Uint256::from(FEE_PRECISION)
                * Uint256::from(SHARE_PRICE_SCALING_FACTOR);
        assert_eq!(num, expected_num);

        let expected_denom = Uint256::from(shares_supply) * Uint256::from(FEE_PRECISION);
        assert_eq!(denom, expected_denom);

        // verify that the share price is 2.205 INJ
        let share_price = num / denom;
        assert_eq!(share_price, Uint256::from(2205000000000000000u64));
    }

    #[test]
    fn test_convert_to_shares() {
        let inj_amount = Uint128::new(ONE_INJ);
        let share_price_num = Uint256::from(ONE_INJ);
        let share_price_denom = Uint256::one();

        let result = convert_to_shares(inj_amount, share_price_num, share_price_denom).unwrap();
        assert_eq!(result, Uint128::new(ONE_INJ));
    }

    #[test]
    fn test_convert_to_shares_higher_share_price() {
        let inj_amount = Uint128::new(ONE_INJ);
        let share_price_num =
            Uint256::from_str("200000000000000000000000000000000000000000").unwrap(); // 2.0 share price
        let share_price_denom = Uint256::from(100000000000000000000000u128);

        let result = convert_to_shares(inj_amount, share_price_num, share_price_denom).unwrap();
        assert_eq!(result, Uint128::new(500000000000000000)); // 0.5 TruINJ
    }

    #[test]
    fn test_convert_to_shares_large_amount() {
        let inj_amount = Uint128::new(100000000000000000000000000); // 100,000,000 INJ
        let share_price_num =
            Uint256::from_str("20000000000000000000000000000000000000000000000000000").unwrap();
        let share_price_denom = Uint256::from(10000000000000000000000000000000000u128);
        // This should cause an overflow in the multiplication since it will equal
        // 100000000000000000000000000 * 10000000000000000000000000000000000 * 1e18 i.e. 1e78
        // but due to the muldiv this will succeed
        let result = convert_to_shares(inj_amount, share_price_num, share_price_denom).unwrap();
        assert_eq!(result, Uint128::new(50000000000000000000000000)); // 50,000,000 TruINJ
    }

    #[test]
    fn test_convert_to_shares_zero_amount() {
        let inj_amount = Uint128::zero();
        let share_price_num = Uint256::from(ONE_INJ);
        let share_price_denom = Uint256::from(ONE_INJ);

        let result = convert_to_shares(inj_amount, share_price_num, share_price_denom).unwrap();
        assert_eq!(result, Uint128::zero());
    }

    #[test]
    fn test_convert_to_shares_overflow() {
        let inj_amount = Uint128::new(ONE_INJ);
        let share_price_num = Uint256::from_str("20000000000").unwrap();
        let share_price_denom =
            Uint256::from_str("200000000000000000000000000000000000000000000000000").unwrap();

        let err = convert_to_shares(inj_amount, share_price_num, share_price_denom).unwrap_err();
        println!("err: {:?}", err);
        assert_eq!(
            err,
            ContractError::Overflow(ConversionOverflowError {
                source_type: "Uint256",
                target_type: "Uint128"
            })
        );
    }

    #[test]
    fn test_convert_to_shares_zero_share_price_num() {
        let inj_amount = Uint128::new(ONE_INJ);
        let share_price_num = Uint256::zero();
        let share_price_denom = Uint256::from(ONE_INJ);

        let err = convert_to_shares(inj_amount, share_price_num, share_price_denom).unwrap_err();
        assert_eq!(err, ContractError::ZeroDiv(DivideByZeroError));
    }

    #[test]
    fn test_convert_to_assets_with_initial_share_price() {
        let shares = 123u128;
        let share_price_num = Uint256::one() * Uint256::from(SHARE_PRICE_SCALING_FACTOR);
        let share_price_denom = Uint256::one();
        let result = convert_to_assets(shares, share_price_num, share_price_denom, false).unwrap();
        assert_eq!(result, 123);
    }

    #[test]
    fn test_convert_to_assets_with_round_share_price() {
        let shares = 124u128;
        let share_price_num = Uint256::from(1024u64) * Uint256::from(SHARE_PRICE_SCALING_FACTOR);
        let share_price_denom = Uint256::from(512u64);
        let result = convert_to_assets(shares, share_price_num, share_price_denom, false).unwrap();
        assert_eq!(result, 248);

        let result = convert_to_assets(shares, share_price_num, share_price_denom, true).unwrap();
        assert_eq!(result, 248);
    }

    #[test]
    fn test_convert_to_assets_with_odd_share_price() {
        let shares = 124u128;
        let share_price_num =
            Uint256::from(200u64) * Uint256::from(SHARE_PRICE_SCALING_FACTOR) + Uint256::from(1u64);
        let share_price_denom = Uint256::from(100u64);
        let result = convert_to_assets(shares, share_price_num, share_price_denom, false).unwrap();
        assert_eq!(result, 248);

        let result = convert_to_assets(shares, share_price_num, share_price_denom, true).unwrap();
        assert_eq!(result, 249);
    }

    #[test]
    fn test_convert_to_assets_large_numbers() {
        // 100,000,000 TruINJ
        let shares = 100_000_000 * ONE_INJ;

        // share price of 2.0 with very large numerator and denominator
        let share_price_num = Uint256::from(20_000_000_000_000_000 * ONE_INJ)
            .checked_mul(Uint256::from(SHARE_PRICE_SCALING_FACTOR))
            .unwrap();
        let share_price_denom = Uint256::from(10_000_000_000_000_000 * ONE_INJ);

        // verify assets are 200,000,000 INJ
        let assets = convert_to_assets(shares, share_price_num, share_price_denom, false).unwrap();
        assert_eq!(assets, 2 * shares);
    }
}
