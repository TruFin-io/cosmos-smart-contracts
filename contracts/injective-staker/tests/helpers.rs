use cosmwasm_std::{
    coin, testing::mock_env, to_json_binary, Addr, CosmosMsg, Decimal, Empty, StdResult, Validator,
    WasmMsg,
};
use cosmwasm_std::{
    Attribute, DelegationResponse, DistributionMsg, Event, FullDelegation, StakingQuery, Uint128,
    Uint256, Uint512,
};
use cw20::BalanceResponse;
use cw_controllers::{Claim, ClaimsResponse};
use cw_multi_test::error::AnyError;
use cw_multi_test::{
    App, AppBuilder, AppResponse, Contract, ContractWrapper, Executor, IntoBech32, StakingInfo,
};
use injective_staker::constants::INJ;
use injective_staker::contract::{execute, instantiate, query};
use injective_staker::msg::{
    ExecuteMsg, GetAllocationsResponse, GetClaimableAmountResponse, GetCurrentUserStatusResponse,
    GetDistributionAmountsResponse, GetIsAgentResponse, GetIsBlacklistedResponse,
    GetIsWhitelistedResponse, GetMaxWithdrawResponse, GetSharePriceResponse, GetStakerInfoResponse,
    GetTotalAllocatedResponse, GetTotalRewardsResponse, GetTotalStakedResponse,
    GetTotalSupplyResponse, InstantiateMsg, QueryMsg,
};
use injective_staker::state::{Allocation, UserStatus};
use injective_staker::SHARE_PRICE_SCALING_FACTOR;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// StakerContract is a wrapper around Addr that provides helper for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StakerContract(pub Addr);

impl StakerContract {
    pub fn simple_staker_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query);
        Box::new(contract)
    }

    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg: to_json_binary(&msg.into()).unwrap(),
            funds: vec![],
        }
        .into())
    }
}

pub fn mock_app_with_validator() -> (App, Addr) {
    let validator_addr = "default-validator".into_bech32();
    let validator_interest_rate: Decimal = Decimal::percent(5);
    let validator_commission: Decimal = Decimal::percent(2);

    let validator = Validator::new(
        validator_addr.to_string(),
        validator_commission,
        Decimal::percent(100),
        Decimal::percent(1),
    );

    let app = AppBuilder::new().build(|router, api, storage| {
        router
            .staking
            .setup(
                storage,
                StakingInfo {
                    bonded_denom: INJ.to_string(),
                    unbonding_time: 21 * 24 * 60 * 60, //  21 days
                    apr: validator_interest_rate,
                },
            )
            .unwrap();
        router
            .staking
            .add_validator(api, storage, &mock_env().block, validator)
            .unwrap();
    });

    (app, validator_addr)
}

pub fn instantiate_staker(admin: Addr, treasury: Addr) -> (App, StakerContract, Addr) {
    let (mut app, validator_addr) = mock_app_with_validator();
    let code_id = app.store_code(contract_wrapper());

    let msg = InstantiateMsg {
        treasury,
        default_validator: validator_addr.clone(),
    };
    let staker_contract_addr = app
        .instantiate_contract(code_id, admin, &msg, &[], "staker-contract", None)
        .unwrap();

    let staker_contract = StakerContract(staker_contract_addr);

    (app, staker_contract, validator_addr)
}

pub fn instantiate_staker_with_min_deposit(
    admin: Addr,
    treasury: Addr,
    min_deposit: u128,
) -> (App, Addr, Addr) {
    let (mut app, staker_contract, validator_addr) = instantiate_staker(admin.clone(), treasury);

    // set min deposit
    set_min_deposit_for_test_overflow(
        &mut app,
        staker_contract.addr().to_string(),
        admin,
        min_deposit,
    );

    (app, staker_contract.addr(), validator_addr)
}

pub fn instantiate_staker_with_min_deposit_and_initial_stake(
    admin: Addr,
    treasury: Addr,
    min_deposit: u128,
    initial_stake: u128,
) -> (App, Addr, Addr) {
    let (mut app, staker_contract, validator_addr) =
        instantiate_staker_with_min_deposit(admin.clone(), treasury, min_deposit);

    if initial_stake > 0 {
        // mint tokens to trufin and perform the initial stake
        let trufin: Addr = "trufin".into_bech32();
        mint_inj(&mut app, &trufin, initial_stake);
        whitelist_user(&mut app, &staker_contract, &admin, &trufin);
        stake(&mut app, &trufin, &staker_contract, initial_stake).unwrap();
    }

    (app, staker_contract, validator_addr)
}

// Helper function to add a validator to the app
pub fn add_validator_to_app(app: &mut App, validator_addr: String) {
    let validator = Validator::new(
        validator_addr,
        Decimal::percent(2),
        Decimal::percent(100),
        Decimal::percent(1),
    );

    app.init_modules(|router, api, storage| {
        router
            .staking
            .add_validator(api, storage, &mock_env().block, validator)
            .unwrap();
    });
}

pub fn contract_wrapper() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

pub fn mint_inj(app: &mut App, addr: &Addr, amount: u128) {
    app.sudo(cw_multi_test::SudoMsg::Bank(
        cw_multi_test::BankSudo::Mint {
            to_address: addr.to_string(),
            amount: vec![coin(amount, INJ)],
        },
    ))
    .unwrap();
}

pub fn query_truinj_balance(app: &App, addr: &Addr, contract_address: &Addr) -> u128 {
    let sender_balance: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            contract_address.clone(),
            &QueryMsg::Balance {
                address: addr.to_string(),
            },
        )
        .unwrap();
    sender_balance.balance.into()
}

pub fn query_truinj_supply(app: &App, contract_addr: &Addr) -> u128 {
    let total_supply: GetTotalSupplyResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::GetTotalSupply {})
        .unwrap();

    total_supply.total_supply.u128()
}

pub fn query_inj_balance(app: &App, address: &Addr) -> u128 {
    let balance = app.wrap().query_balance(address.clone(), "inj").unwrap();
    balance.amount.u128()
}

pub fn add_validator(
    app: &mut App,
    sender: Addr,
    contract_addr: &Addr,
    validator: Addr,
) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::AddValidator { validator };

    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![],
    };
    app.execute(sender, cosmos_msg.into())
}

pub fn stake(
    app: &mut App,
    sender: &Addr,
    contract_addr: &Addr,
    amount: u128,
) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::Stake {};

    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![coin(amount, INJ)],
    };
    app.execute(sender.clone(), cosmos_msg.into())
}

pub fn stake_to_specific_validator(
    app: &mut App,
    sender: &Addr,
    contract_addr: &Addr,
    amount: u128,
    validator_addr: &Addr,
) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::StakeToSpecificValidator {
        validator_addr: validator_addr.clone(),
    };

    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![coin(amount, INJ)],
    };
    app.execute(sender.clone(), cosmos_msg.into())
}

// Mimics rewards being automatically sweeped when staking
pub fn stake_when_rewards_accrued(
    app: &mut App,
    sender: &Addr,
    contract_addr: &Addr,
    amount: u128,
    validator_addr: &Addr,
) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::StakeToSpecificValidator {
        validator_addr: validator_addr.clone(),
    };
    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![coin(amount, INJ)],
    };

    let stake_res = app.execute(sender.clone(), cosmos_msg.into());

    let collect_rewards_msg: CosmosMsg<_> =
        CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
            validator: validator_addr.to_string(),
        });
    app.execute(contract_addr.clone(), collect_rewards_msg)
        .unwrap();

    stake_res
}

pub fn unstake(
    app: &mut App,
    sender: &Addr,
    contract_addr: &Addr,
    amount: u128,
) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::Unstake {
        amount: amount.into(),
    };

    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![],
    };
    app.execute(sender.clone(), cosmos_msg.into())
}

pub fn unstake_when_rewards_accrue(
    app: &mut App,
    sender: &Addr,
    contract_addr: &Addr,
    amount: u128,
    validator_addr: &Addr,
) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::UnstakeFromSpecificValidator {
        validator_addr: validator_addr.clone(),
        amount: amount.into(),
    };

    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![],
    };
    let collect_rewards_msg: CosmosMsg<_> =
        CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
            validator: validator_addr.to_string(),
        });

    let delegation = get_delegation(app, contract_addr.clone().to_string(), validator_addr);

    let unstake_res = app.execute(sender.clone(), cosmos_msg.into());
    let collect_rewards_res = app.execute(contract_addr.clone(), collect_rewards_msg);

    // if the res failed everything was unstaked from the validator and on cw_multi_test rewards are then lost
    // hence, we mint the contract the rewards instead
    if collect_rewards_res.is_err() {
        app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: contract_addr.to_string(),
                amount: delegation.accumulated_rewards,
            },
        ))
        .unwrap();
    }
    unstake_res
}

pub fn set_up_allocation(
    app: &mut App,
    owner: &Addr,
    contract_addr: &Addr,
    inj_to_stake: u128,
    user: &Addr,
) {
    mint_inj(app, user, inj_to_stake);
    whitelist_user(app, contract_addr, owner, user);
    stake(app, user, contract_addr, inj_to_stake).unwrap();
}

pub fn set_up_test_allocation(
    app: &mut App,
    owner: &Addr,
    contract_addr: &Addr,
    user: &Addr,
    recipient: &Addr,
    amount: u128,
) {
    mint_inj(app, user, amount);
    whitelist_user(app, contract_addr, owner, user);
    stake(app, user, contract_addr, amount).unwrap();

    // call TestAllocate to bypass the min allocation check
    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&ExecuteMsg::TestAllocate {
            recipient: recipient.clone(),
            amount: amount.into(),
        })
        .unwrap(),
        funds: vec![],
    };
    let res = app.execute(user.clone(), cosmos_msg.into());
    assert!(res.is_ok());
}

pub fn allocate(
    app: &mut App,
    sender: &Addr,
    contract_addr: &Addr,
    amount: u128,
    recipient: &Addr,
) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::Allocate {
        recipient: recipient.clone(),
        amount: amount.into(),
    };

    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![],
    };
    app.execute(sender.clone(), cosmos_msg.into())
}

pub fn deallocate(
    app: &mut App,
    sender: &Addr,
    contract_addr: &Addr,
    amount: u128,
    recipient: &Addr,
) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::Deallocate {
        recipient: recipient.clone(),
        amount: amount.into(),
    };

    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![],
    };
    app.execute(sender.clone(), cosmos_msg.into())
}

pub fn claim(app: &mut App, sender: &Addr, contract_addr: &Addr) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::Claim {};

    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![],
    };
    app.execute(sender.clone(), cosmos_msg.into())
}

pub fn enable_validator(
    app: &mut App,
    sender: Addr,
    contract_addr: &Addr,
    validator: Addr,
) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::EnableValidator { validator };

    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![],
    };
    app.execute(sender, cosmos_msg.into())
}

pub fn disable_validator(
    app: &mut App,
    sender: Addr,
    contract_addr: &Addr,
    validator: Addr,
) -> Result<AppResponse, AnyError> {
    let msg = ExecuteMsg::DisableValidator { validator };

    let cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![],
    };
    app.execute(sender, cosmos_msg.into())
}

pub fn wasm_execute_msg(staking_contract: &Addr, msg: &ExecuteMsg) -> WasmMsg {
    WasmMsg::Execute {
        contract_addr: staking_contract.to_string(),
        msg: to_json_binary(&msg).unwrap(),
        funds: vec![],
    }
}

pub fn query_is_agent(app: &App, agent: &Addr, contract: &Addr) -> bool {
    let is_agent_response: GetIsAgentResponse = app
        .wrap()
        .query_wasm_smart(
            contract,
            &QueryMsg::IsAgent {
                agent: agent.clone(),
            },
        )
        .unwrap();
    is_agent_response.is_agent
}

pub fn claimable_amount(app: &App, user: &Addr, contract: &Addr) -> Uint128 {
    let claimable_amount_response: GetClaimableAmountResponse = app
        .wrap()
        .query_wasm_smart(
            contract,
            &QueryMsg::GetClaimableAmount { user: user.clone() },
        )
        .unwrap();
    claimable_amount_response.claimable_amount
}

pub fn query_staker_info(app: &App, contract: &Addr) -> GetStakerInfoResponse {
    app.wrap()
        .query_wasm_smart(contract, &QueryMsg::GetStakerInfo {})
        .unwrap()
}

pub fn add_agent(app: &mut App, staker_contract: &Addr, owner: &Addr, new_agent: &Addr) {
    let response = app.execute(
        owner.clone(),
        wasm_execute_msg(
            staker_contract,
            &ExecuteMsg::AddAgent {
                agent: new_agent.clone(),
            },
        )
        .into(),
    );
    assert!(response.is_ok());
}

pub fn whitelist_user(app: &mut App, contract: &Addr, agent: &Addr, user: &Addr) {
    let response = app.execute(
        agent.clone(),
        wasm_execute_msg(
            contract,
            &ExecuteMsg::AddUserToWhitelist { user: user.clone() },
        )
        .into(),
    );
    assert!(response.is_ok());
}

pub fn blacklist_user(app: &mut App, contract: &Addr, agent: &Addr, user: &Addr) {
    let response = app.execute(
        agent.clone(),
        wasm_execute_msg(
            contract,
            &ExecuteMsg::AddUserToBlacklist { user: user.clone() },
        )
        .into(),
    );
    assert!(response.is_ok());
}

pub fn clear_whitelist_status(app: &mut App, staker_addr: &Addr, agent: &Addr, user: &Addr) {
    let response = app.execute(
        agent.clone(),
        wasm_execute_msg(
            staker_addr,
            &ExecuteMsg::ClearUserStatus { user: user.clone() },
        )
        .into(),
    );
    assert!(response.is_ok());
}

pub fn pause(app: &mut App, contract: &Addr, owner: &Addr) {
    let response = app.execute(
        owner.clone(),
        wasm_execute_msg(contract, &ExecuteMsg::Pause).into(),
    );
    assert!(response.is_ok());
}

pub fn unpause(app: &mut App, contract: &Addr, owner: &Addr) {
    let response = app.execute(
        owner.clone(),
        wasm_execute_msg(contract, &ExecuteMsg::Unpause).into(),
    );
    assert!(response.is_ok());
}

pub fn is_user_whitelisted(app: &App, user: &Addr, contract: &Addr) -> bool {
    let response: GetIsWhitelistedResponse = app
        .wrap()
        .query_wasm_smart(contract, &QueryMsg::IsWhitelisted { user: user.clone() })
        .unwrap();
    response.is_whitelisted
}

pub fn is_user_blacklisted(app: &App, user: &Addr, contract: &Addr) -> bool {
    let response: GetIsBlacklistedResponse = app
        .wrap()
        .query_wasm_smart(contract, &QueryMsg::IsBlacklisted { user: user.clone() })
        .unwrap();
    response.is_blacklisted
}

pub fn query_user_status(app: &App, user: &Addr, contract: &Addr) -> UserStatus {
    let response: GetCurrentUserStatusResponse = app
        .wrap()
        .query_wasm_smart(
            contract,
            &QueryMsg::GetCurrentUserStatus { user: user.clone() },
        )
        .unwrap();
    response.user_status
}

pub fn assert_error(response: Result<AppResponse, AnyError>, expected_error_msg: &str) {
    let error = response.unwrap_err();
    let error_source = error.source().unwrap();
    assert_eq!(error_source.to_string(), expected_error_msg);
}

pub fn assert_event_with_attributes(
    events: &[Event],
    expected_event_name: &str,
    expected_attributes: Vec<Attribute>,
    contract_address: Addr,
) {
    let emitted_event = events
        .iter()
        .find(|p: &&_| p.ty == expected_event_name)
        .expect("Event not emitted");

    // Assert the event name
    assert_eq!(emitted_event.ty, expected_event_name);

    // add the _contract_address attribute that is always present
    let mut expected_attributes_with_address = vec![Attribute {
        key: "_contract_address".to_string(),
        value: contract_address.to_string(),
    }];
    expected_attributes_with_address.extend(expected_attributes);

    // assert the attributes
    assert_eq!(emitted_event.attributes, expected_attributes_with_address);
}

pub fn get_total_staked(app: &App, contract_addr: &Addr) -> Uint128 {
    let total_staked: GetTotalStakedResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::GetTotalStaked {})
        .unwrap();
    total_staked.total_staked
}

pub fn get_allocations(app: &App, contract_addr: &Addr, user: &Addr) -> Vec<Allocation> {
    let allocations: GetAllocationsResponse = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::GetAllocations { user: user.clone() },
        )
        .unwrap();
    allocations.allocations
}

pub fn get_total_allocated(
    app: &App,
    contract_addr: &Addr,
    user: &Addr,
) -> (Uint128, Uint256, Uint256) {
    let total_allocated: GetTotalAllocatedResponse = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::GetTotalAllocated { user: user.clone() },
        )
        .unwrap();
    (
        total_allocated.total_allocated_amount,
        total_allocated.total_allocated_share_price_num,
        total_allocated.total_allocated_share_price_denom,
    )
}

pub fn get_total_rewards(app: &App, contract_addr: &Addr) -> Uint128 {
    let total_rewards: GetTotalRewardsResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::GetTotalRewards {})
        .unwrap();
    total_rewards.total_rewards
}

pub fn get_delegation(app: &App, contract_addr: String, validator_addr: &Addr) -> FullDelegation {
    let delegation_response: DelegationResponse = app
        .wrap()
        .query(
            &StakingQuery::Delegation {
                delegator: contract_addr,
                validator: validator_addr.clone().to_string(),
            }
            .into(),
        )
        .unwrap();

    delegation_response.delegation.unwrap()
}

pub fn get_staker_info(app: &App, contract_addr: &Addr) -> GetStakerInfoResponse {
    app.wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::GetStakerInfo {})
        .unwrap()
}

pub fn get_claimable_assets(app: &App, contract_addr: &Addr, user: &Addr) -> Vec<Claim> {
    let response: ClaimsResponse = app
        .wrap()
        .query_wasm_smart(
            contract_addr,
            &QueryMsg::GetClaimableAssets { user: user.clone() },
        )
        .unwrap();
    response.claims
}

pub fn get_share_price(app: &App, contract_addr: &Addr) -> u128 {
    let response: GetSharePriceResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetSharePrice {})
        .unwrap();

    response
        .numerator
        .checked_div(response.denominator)
        .unwrap()
        .to_string()
        .parse::<u128>()
        .unwrap()
}

pub fn get_share_price_num_denom(app: &App, contract_addr: &Addr) -> (Uint256, Uint256) {
    let response: GetSharePriceResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetSharePrice {})
        .unwrap();
    (response.numerator, response.denominator)
}

pub fn get_max_withdraw(app: &App, staker_addr: &Addr, user: &Addr) -> u128 {
    let response: GetMaxWithdrawResponse = app
        .wrap()
        .query_wasm_smart(
            staker_addr.clone(),
            &QueryMsg::GetMaxWithdraw { user: user.clone() },
        )
        .unwrap();
    response.max_withdraw.u128()
}

pub fn get_distribution_amounts(
    app: &App,
    staker_addr: &Addr,
    distributor: &Addr,
    recipient: Option<&Addr>,
) -> (u128, u128, u128) {
    let response: GetDistributionAmountsResponse = app
        .wrap()
        .query_wasm_smart(
            staker_addr.clone(),
            &QueryMsg::GetDistributionAmounts {
                distributor: distributor.clone(),
                recipient: recipient.cloned(),
            },
        )
        .unwrap();

    (
        response.inj_amount.u128(),
        response.truinj_amount.u128(),
        response.distribution_fee.u128(),
    )
}

pub fn move_days_forward(app: &mut App, days: u64) {
    app.update_block(|block| {
        block.time = block.time.plus_seconds(days * 24 * 60 * 60);
    });
}

pub fn set_min_deposit_for_test_overflow(
    app: &mut App,
    contract_addr: String,
    owner: Addr,
    min_deposit: u128,
) {
    let msg = WasmMsg::Execute {
        contract_addr,
        msg: to_json_binary(&ExecuteMsg::SetMinimumDeposit {
            new_min_deposit: Uint128::new(min_deposit),
        })
        .unwrap(),
        funds: vec![],
    };
    app.execute(owner, msg.into()).unwrap();
}

pub fn set_fee(app: &mut App, contract_addr: &Addr, owner: &Addr, new_fee: u16) {
    let msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&ExecuteMsg::SetFee { new_fee }).unwrap(),
        funds: vec![],
    };

    let response = app.execute(owner.clone(), msg.into());
    assert!(response.is_ok());
}

pub fn set_dist_fee(app: &mut App, contract_addr: &Addr, owner: &Addr, new_distribution_fee: u16) {
    let msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&ExecuteMsg::SetDistributionFee {
            new_distribution_fee,
        })
        .unwrap(),
        funds: vec![],
    };

    let response = app.execute(owner.clone(), msg.into());
    assert!(response.is_ok());
}

pub fn convert_to_assets(
    shares: u128,
    share_price_num: Uint256,
    share_price_denom: Uint256,
) -> u128 {
    let assets = Uint512::from(shares) * Uint512::from(share_price_num)
        / Uint512::from(share_price_denom)
            .checked_mul(Uint512::from(SHARE_PRICE_SCALING_FACTOR))
            .unwrap();

    Uint128::try_from(assets).unwrap().u128()
}

pub fn transfer_truinj(
    app: &mut App,
    contract_addr: &Addr,
    sender: &Addr,
    recipient: &Addr,
    amount: u128,
) {
    let msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount: Uint128::new(amount),
        })
        .unwrap(),
        funds: vec![],
    };
    let res = app.execute(sender.clone(), msg.into());
    assert!(res.is_ok());
}
