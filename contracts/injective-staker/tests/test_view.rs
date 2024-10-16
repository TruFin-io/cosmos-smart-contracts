pub mod helpers;

#[cfg(test)]
mod view {

    use cosmwasm_std::{Addr, Uint128, Uint256};
    use cw_multi_test::{Executor, IntoBech32};
    use helpers::{contract_wrapper, mint_inj, mock_app_with_validator, stake};
    use injective_staker::constants::INJ;
    use injective_staker::msg::{
        GetDistributionAmountsResponse, GetMaxWithdrawResponse, GetTotalAssetsResponse,
    };
    use injective_staker::{
        msg::{GetSharePriceResponse, GetTotalSupplyResponse, InstantiateMsg, QueryMsg},
        SHARE_PRICE_SCALING_FACTOR,
    };
    use injective_staker::{FEE_PRECISION, ONE_INJ};

    use crate::helpers::{
        self, add_validator, add_validator_to_app, allocate, claimable_amount, convert_to_assets,
        get_delegation, get_max_withdraw, get_share_price, get_share_price_num_denom,
        get_total_rewards, get_total_staked, instantiate_staker,
        instantiate_staker_with_min_deposit, instantiate_staker_with_min_deposit_and_initial_stake,
        move_days_forward, set_dist_fee, set_up_allocation, unstake, whitelist_user,
    };

    #[test]
    fn test_get_total_supply() {
        let (mut app, validator_addr) = mock_app_with_validator();
        let code_id = app.store_code(contract_wrapper());

        // instantiate the contract
        let owner = "owner".into_bech32();
        let msg = InstantiateMsg {
            treasury: "treasury".into_bech32(),
            default_validator: validator_addr,
        };

        let contract_addr = app
            .instantiate_contract(code_id, owner.clone(), &msg, &[], "staker-contract", None)
            .unwrap();

        let anyone: Addr = "anyone".into_bech32();

        let pre_total_supply: GetTotalSupplyResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::GetTotalSupply {})
            .unwrap();
        assert!(pre_total_supply.total_supply.is_zero());

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        // ensure total supply was updated
        let post_total_supply: GetTotalSupplyResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::GetTotalSupply {})
            .unwrap();
        assert!(post_total_supply.total_supply.u128() == inj_to_mint);
    }

    #[test]
    fn test_get_total_staked_without_staking_multi_validator() {
        let (mut app, validator_addr) = mock_app_with_validator();
        let code_id = app.store_code(contract_wrapper());

        // instantiate the contract
        let owner = "owner".into_bech32();
        let msg = InstantiateMsg {
            treasury: "treasury".into_bech32(),
            default_validator: validator_addr,
        };

        let contract_addr = app
            .instantiate_contract(code_id, owner.clone(), &msg, &[], "staker-contract", None)
            .unwrap();

        // add a second validator:
        let validator = "validator".into_bech32();
        add_validator_to_app(&mut app, validator.to_string());
        add_validator(&mut app, owner, &contract_addr, validator).unwrap();

        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        assert!(total_staked.is_zero());
        assert!(total_rewards.is_zero());
    }

    #[test]
    fn test_get_total_staked_without_staking() {
        let (mut app, validator_addr) = mock_app_with_validator();
        let code_id = app.store_code(contract_wrapper());

        // instantiate the contract
        let owner = "owner".into_bech32();
        let msg = InstantiateMsg {
            treasury: "treasury".into_bech32(),
            default_validator: validator_addr,
        };

        let contract_addr = app
            .instantiate_contract(code_id, owner, &msg, &[], "staker-contract", None)
            .unwrap();

        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        assert!(total_staked.is_zero());
        assert!(total_rewards.is_zero());
    }

    #[test]
    fn test_get_total_staked_with_staking() {
        let (mut app, validator_addr) = mock_app_with_validator();
        let code_id = app.store_code(contract_wrapper());

        // instantiate the contract
        let owner = "owner".into_bech32();
        let msg = InstantiateMsg {
            treasury: "treasury".into_bech32(),
            default_validator: validator_addr,
        };

        let contract_addr = app
            .instantiate_contract(code_id, owner.clone(), &msg, &[], "staker-contract", None)
            .unwrap();

        // mint INJ tokens to the 'anyone' user
        let anyone: Addr = "anyone".into_bech32();
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        assert!(total_staked.u128() == inj_to_mint);
        assert!(total_rewards.is_zero());
    }

    #[test]
    fn test_get_total_staked_with_multi_validators() {
        let (mut app, validator_addr) = mock_app_with_validator();
        let code_id = app.store_code(contract_wrapper());

        // instantiate the contract
        let owner = "owner".into_bech32();
        let msg = InstantiateMsg {
            treasury: "treasury".into_bech32(),
            default_validator: validator_addr,
        };

        let contract_addr = app
            .instantiate_contract(code_id, owner.clone(), &msg, &[], "staker-contract", None)
            .unwrap();

        // add a second validator:
        let validator = "validator".into_bech32();
        add_validator_to_app(&mut app, validator.to_string());
        add_validator(&mut app, owner.clone(), &contract_addr, validator).unwrap();

        let anyone: Addr = "anyone".into_bech32();
        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        assert!(total_staked.u128() == inj_to_mint);
        assert!(total_rewards.is_zero());
    }

    #[test]
    fn test_get_total_rewards_with_rewards_accruing() {
        // instantiate the contract
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, validator_addr) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 100000);

        let anyone: Addr = "anyone".into_bech32();
        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 1000000000000; // low enough INJ amount to not cause overflow error.
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        assert!(total_staked.u128() == inj_to_mint);
        assert!(total_rewards.is_zero());

        // simulate passage of time and reward accrual
        move_days_forward(&mut app, 1);

        // query total staked and rewards after reward distribution
        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        let delegation = get_delegation(&app, contract_addr.to_string(), &validator_addr);
        let acc_rewards = delegation
            .accumulated_rewards
            .iter()
            .find(|coin| coin.denom == INJ)
            .expect("INJ rewards not found");

        assert!(total_staked.u128() == inj_to_mint);
        assert!(total_rewards == acc_rewards.amount);
    }

    #[test]
    fn test_get_total_rewards_with_multiple_validators() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, validator_addr) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 100000);

        // add a second validator:
        let validator = "validator".into_bech32();
        add_validator_to_app(&mut app, validator.to_string());
        add_validator(&mut app, owner.clone(), &contract_addr, validator).unwrap();

        let anyone: Addr = "anyone".into_bech32();
        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 1000000000000; // low enough INJ amount to not cause overflow error.
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        assert!(total_staked.u128() == inj_to_mint);
        assert!(total_rewards.is_zero());

        // simulate passage of time and reward accrual
        move_days_forward(&mut app, 1);

        // query total staked and rewards after reward distribution
        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        // fetch total rewards from validator
        let delegation = get_delegation(&app, contract_addr.to_string(), &validator_addr);
        let acc_rewards = delegation
            .accumulated_rewards
            .iter()
            .find(|coin| coin.denom == INJ)
            .expect("INJ rewards not found");

        assert!(total_staked.u128() == inj_to_mint);
        assert!(total_rewards == acc_rewards.amount);
    }

    #[test]
    fn test_get_total_assets() {
        let (mut app, staker_contract, _) =
            instantiate_staker("owner".into_bech32(), "treasury".into_bech32());

        // mint INJ tokens to the staker contract
        let staker_assets = 1234 * ONE_INJ;
        mint_inj(&mut app, &staker_contract.addr(), staker_assets);

        let response: GetTotalAssetsResponse = app
            .wrap()
            .query_wasm_smart(staker_contract.addr(), &QueryMsg::GetTotalAssets {})
            .unwrap();

        assert_eq!(response.total_assets.u128(), staker_assets);
    }

    #[test]
    fn test_get_share_price_when_no_shares_exist() {
        let (app, staker_contract, _) =
            instantiate_staker("owner".into_bech32(), "treasury".into_bech32());

        let response: GetSharePriceResponse = app
            .wrap()
            .query_wasm_smart(staker_contract.addr(), &QueryMsg::GetSharePrice {})
            .unwrap();

        assert_eq!(
            response.numerator,
            Uint256::from(SHARE_PRICE_SCALING_FACTOR)
        );
        assert_eq!(response.denominator, Uint256::from(1u64));
    }

    #[test]
    fn test_get_share_price_increases_with_rewards() {
        let owner = "owner".into_bech32();

        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 100000);

        // stake some INJ
        let alice: Addr = "alice".into_bech32();
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        let stake_amount = 1000000000000;
        mint_inj(&mut app, &alice, stake_amount);
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // verify initial share price
        let share_price_day_0 = get_share_price(&app, &staker_addr);
        assert_eq!(share_price_day_0, SHARE_PRICE_SCALING_FACTOR);

        // accrue rewards and verify share price increases
        move_days_forward(&mut app, 1);
        let share_price_day_1 = get_share_price(&app, &staker_addr);
        assert!(share_price_day_1 > share_price_day_0);

        // accrue rewards and verify share price increases
        move_days_forward(&mut app, 1);
        let share_price_day_2 = get_share_price(&app, &staker_addr);
        assert!(share_price_day_2 > share_price_day_1);
    }

    #[test]
    fn test_get_share_price_increases_with_multiple_validators() {
        let owner = "owner".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 100000);

        // add a second validator
        let second_validator = "second-validator".into_bech32();
        add_validator_to_app(&mut app, second_validator.to_string());
        add_validator(&mut app, owner.clone(), &staker_contract, second_validator).unwrap();

        // stake some INJ
        let alice: Addr = "alice".into_bech32();
        whitelist_user(&mut app, &staker_contract, &owner, &alice);
        let stake_amount = 1000000000000;
        mint_inj(&mut app, &alice, stake_amount);
        stake(&mut app, &alice, &staker_contract, stake_amount).unwrap();

        // verify initial share price
        let share_price_day_0 = get_share_price(&app, &staker_contract);
        assert_eq!(share_price_day_0, SHARE_PRICE_SCALING_FACTOR);

        // accrue rewards and verify share price increases
        move_days_forward(&mut app, 1);
        let share_price_day_1 = get_share_price(&app, &staker_contract);
        assert!(share_price_day_1 > share_price_day_0);

        // accrue rewards and verify share price increases
        move_days_forward(&mut app, 1);
        let share_price_day_2 = get_share_price(&app, &staker_contract);
        assert!(share_price_day_2 > share_price_day_1);
    }

    #[test]
    fn test_max_withdraw_with_no_deposits() {
        let owner = "owner".into_bech32();
        let (app, staker_contract, _) = instantiate_staker(owner, "treasury".into_bech32());

        let alice = "alice".into_bech32();
        let response: GetMaxWithdrawResponse = app
            .wrap()
            .query_wasm_smart(
                staker_contract.addr(),
                &QueryMsg::GetMaxWithdraw { user: alice },
            )
            .unwrap();

        assert_eq!(response.max_withdraw.u128(), 0);
    }

    #[test]
    fn test_max_withdraw_matches_deposits_when_no_rewards() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            100_000,
        );

        let alice = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // alice stakes some inj
        let first_stake = 70_000;
        stake(&mut app, &alice, &staker_addr, first_stake).unwrap();

        let response: GetMaxWithdrawResponse = app
            .wrap()
            .query_wasm_smart(
                staker_addr.clone(),
                &QueryMsg::GetMaxWithdraw {
                    user: alice.clone(),
                },
            )
            .unwrap();

        assert_eq!(response.max_withdraw.u128(), first_stake);

        // alice stakes more inj
        let second_stake = 30_000;
        stake(&mut app, &alice, &staker_addr, second_stake).unwrap();

        // verify max withdraw is the sum of the two stakes
        let response: GetMaxWithdrawResponse = app
            .wrap()
            .query_wasm_smart(
                staker_addr.clone(),
                &QueryMsg::GetMaxWithdraw {
                    user: alice.clone(),
                },
            )
            .unwrap();

        assert_eq!(response.max_withdraw.u128(), first_stake + second_stake);
    }

    #[test]
    fn test_max_withdraw_is_greater_than_deposits_when_rewards_accrue() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 100_000);

        // alice stakes some INJ
        let alice = "alice".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // rewards accrue
        move_days_forward(&mut app, 1);

        // verify that max withdraw is greater than the initial stake amount
        let response: GetMaxWithdrawResponse = app
            .wrap()
            .query_wasm_smart(
                staker_addr.clone(),
                &QueryMsg::GetMaxWithdraw {
                    user: alice.clone(),
                },
            )
            .unwrap();

        assert!(response.max_withdraw.u128() > stake_amount);
    }

    #[test]
    fn test_max_withdraw_increases_with_rewards() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 100_000);

        // alice stakes some INJ
        let alice = "alice".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // verify that max withdraw matches the initial stake amount
        let first = get_max_withdraw(&app, &staker_addr, &alice);
        assert_eq!(first, stake_amount);
        // rewards accrue
        move_days_forward(&mut app, 1);

        // verify that max withdraw increased with rewards
        let second = get_max_withdraw(&app, &staker_addr, &alice);
        assert!(second > first);
        // rewards accrue
        move_days_forward(&mut app, 1);

        // verify that max withdraw increased with rewards
        let third = get_max_withdraw(&app, &staker_addr, &alice);
        assert!(third > second);
    }

    #[test]
    fn test_max_withdraw_is_zero_when_all_stake_is_unstaked() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            100_000,
        );

        // alice stakes some inj
        let alice = "alice".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // rewards accrue
        move_days_forward(&mut app, 30);

        // get max withdraw
        let max_withdraw = get_max_withdraw(&app, &staker_addr, &alice);
        println!("max_withdraw: {}", max_withdraw);

        // unstake all
        let _ = unstake(&mut app, &alice, &staker_addr, max_withdraw);

        // verify max withdraw is zero
        let max_withdraw = get_max_withdraw(&app, &staker_addr, &alice);
        assert_eq!(max_withdraw, 0);
    }

    #[test]
    fn test_is_claimable_is_0_when_no_claims_are_claimable() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            100_000,
        );

        // alice stakes some inj
        let alice = "alice".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // rewards accrue
        move_days_forward(&mut app, 30);

        // unstake
        unstake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // time passes but not 21 days
        move_days_forward(&mut app, 20);

        let claimable_amount = claimable_amount(&app, &alice, &staker_addr);
        assert!(claimable_amount.is_zero());
    }

    #[test]
    fn test_is_claimable_is_0_when_no_claims_exist() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            100_000,
        );

        // alice stakes some inj
        let alice = "alice".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        let claimable_amount = claimable_amount(&app, &alice, &staker_addr);
        assert!(claimable_amount.is_zero());
    }

    #[test]
    fn test_is_claimable() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            100_000,
        );

        // alice stakes some inj
        let alice = "alice".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // rewards accrue
        move_days_forward(&mut app, 30);

        // unstake
        unstake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // 21 days pass
        move_days_forward(&mut app, 21);

        let claimable_amount = claimable_amount(&app, &alice, &staker_addr);
        assert!(claimable_amount.u128() == stake_amount);
    }

    #[test]
    fn test_is_claimable_several_claims() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            100_000,
        );

        // alice stakes some inj
        let alice = "alice".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // rewards accrue
        move_days_forward(&mut app, 30);

        // unstake fully over multiple unstakes
        unstake(&mut app, &alice, &staker_addr, 20000).unwrap();

        move_days_forward(&mut app, 10);
        unstake(&mut app, &alice, &staker_addr, 50000).unwrap();
        unstake(&mut app, &alice, &staker_addr, 15000).unwrap();

        move_days_forward(&mut app, 1);
        unstake(&mut app, &alice, &staker_addr, 15000).unwrap();

        move_days_forward(&mut app, 21);

        let claimable_amount = claimable_amount(&app, &alice, &staker_addr);
        assert!(claimable_amount.u128() == stake_amount);
    }

    #[test]
    fn test_is_claimable_several_claims_only_some_claimable() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            100_000,
        );

        // alice stakes some inj
        let alice = "alice".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // rewards accrue
        move_days_forward(&mut app, 30);

        unstake(&mut app, &alice, &staker_addr, 20000).unwrap();

        move_days_forward(&mut app, 20);
        unstake(&mut app, &alice, &staker_addr, 50000).unwrap();
        unstake(&mut app, &alice, &staker_addr, 15000).unwrap();

        move_days_forward(&mut app, 1);
        unstake(&mut app, &alice, &staker_addr, 15000).unwrap();

        let claimable_amount = claimable_amount(&app, &alice, &staker_addr);
        assert!(claimable_amount.u128() == 20000);
    }

    #[test]
    fn test_get_distribution_amounts_to_recipient() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            0,
        );

        // set a 5% distribution fee
        let dist_fee = 500;
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);

        // set up an allocation at the current share price
        let (alloc_share_price_num, alloc_share_price_denom) =
            get_share_price_num_denom(&app, &staker_addr);
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 10 * ONE_INJ;
        set_up_allocation(&mut app, &owner, &staker_addr, 100_000, &distributor);

        allocate(
            &mut app,
            &distributor,
            &staker_addr,
            allocation_amount,
            &recipient,
        )
        .unwrap();

        // accrue rewards
        move_days_forward(&mut app, 30);

        // get the inj, truinj amounts required to distribute the allocation at the current share price
        let (share_price_num, share_price_denom) = get_share_price_num_denom(&app, &staker_addr);

        let dist_amounts_res: GetDistributionAmountsResponse = app
            .wrap()
            .query_wasm_smart(
                staker_addr.clone(),
                &QueryMsg::GetDistributionAmounts {
                    distributor: distributor.clone(),
                    recipient: Some(recipient.clone()),
                },
            )
            .unwrap();

        // calculate the amount of shares required by the distribution before fees
        let amount = Uint256::from(allocation_amount);
        let price_scaling = Uint256::from(SHARE_PRICE_SCALING_FACTOR);
        let lhs = amount * alloc_share_price_denom * price_scaling / alloc_share_price_num;
        let rhs = amount * share_price_denom * price_scaling / share_price_num;
        let shares_to_distribute_before_fees = Uint128::try_from(lhs - rhs).unwrap().u128();

        // calculate the distribution fees in truinj
        let fees = shares_to_distribute_before_fees * dist_fee as u128 / FEE_PRECISION as u128;

        // calculate shares for the distribution after fees, and the equivalent amount of assets
        let shares_to_distribute = shares_to_distribute_before_fees - fees;
        let assets_to_distribute =
            convert_to_assets(shares_to_distribute, share_price_num, share_price_denom);

        // verify the amounts required for the distribution
        assert_eq!(
            dist_amounts_res,
            GetDistributionAmountsResponse {
                truinj_amount: Uint128::from(shares_to_distribute),
                inj_amount: Uint128::from(assets_to_distribute),
                distribution_fee: Uint128::from(fees),
            },
        );
    }

    #[test]
    fn test_get_distribution_amounts_to_recipient_with_no_allocation() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            0,
        );

        // set a 5% distribution fee
        let dist_fee = 500;
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);

        // set up an allocation at the current share price
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 10 * ONE_INJ;
        set_up_allocation(&mut app, &owner, &staker_addr, 100_000, &distributor);
        allocate(
            &mut app,
            &distributor,
            &staker_addr,
            allocation_amount,
            &recipient,
        )
        .unwrap();

        // accrue rewards
        move_days_forward(&mut app, 30);

        // get the distribution amounts for non-existing recipient
        let dist_amount_res: GetDistributionAmountsResponse = app
            .wrap()
            .query_wasm_smart(
                staker_addr.clone(),
                &QueryMsg::GetDistributionAmounts {
                    distributor,
                    recipient: Some("non-recipient".into_bech32()),
                },
            )
            .unwrap();

        // verify the amounts required for the distribution are zero
        assert_eq!(
            dist_amount_res,
            GetDistributionAmountsResponse {
                truinj_amount: Uint128::zero(),
                inj_amount: Uint128::zero(),
                distribution_fee: Uint128::zero(),
            },
        );
    }

    #[test]
    fn test_get_distribution_amounts_to_all() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            0,
        );

        // set a 5% distribution fee
        let dist_fee = 500;
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);
        let distributor = "distributor".into_bech32();
        set_up_allocation(&mut app, &owner, &staker_addr, 100_000, &distributor);

        // set up an allocation at the current share price
        let (first_alloc_share_price_num, first_alloc_share_price_denom) =
            get_share_price_num_denom(&app, &staker_addr);

        let first_recipient = "first-recipient".into_bech32();
        let first_allocation_amount = 10 * ONE_INJ;
        allocate(
            &mut app,
            &distributor,
            &staker_addr,
            first_allocation_amount,
            &first_recipient,
        )
        .unwrap();

        // accrue rewards
        move_days_forward(&mut app, 30);

        let (second_alloc_share_price_num, second_alloc_share_price_denom) =
            get_share_price_num_denom(&app, &staker_addr);

        let second_recipient = "second-recipient".into_bech32();
        let second_allocation_amount = 20 * ONE_INJ;
        allocate(
            &mut app,
            &distributor,
            &staker_addr,
            second_allocation_amount,
            &second_recipient,
        )
        .unwrap();

        // accrue rewards
        move_days_forward(&mut app, 30);

        // get the distribution share price
        let (share_price_num, share_price_denom) = get_share_price_num_denom(&app, &staker_addr);

        // get the amounts for a distribution to all recipients
        let dist_amounts_res: GetDistributionAmountsResponse = app
            .wrap()
            .query_wasm_smart(
                staker_addr.clone(),
                &QueryMsg::GetDistributionAmounts {
                    distributor: distributor.clone(),
                    recipient: None,
                },
            )
            .unwrap();

        // calculate the expected distribution amounts for the first allocation
        let first_amount = Uint256::from(first_allocation_amount);
        let price_scaling = Uint256::from(SHARE_PRICE_SCALING_FACTOR);
        let lhs = first_amount * first_alloc_share_price_denom * price_scaling
            / first_alloc_share_price_num;
        let rhs = first_amount * share_price_denom * price_scaling / share_price_num;
        let shares_to_distribute_before_fees = Uint128::try_from(lhs - rhs).unwrap().u128();

        // calculate the distribution fees in truinj
        let first_fees =
            shares_to_distribute_before_fees * dist_fee as u128 / FEE_PRECISION as u128;

        // calculate shares for the distribution after fees
        let first_shares_to_distribute = shares_to_distribute_before_fees - first_fees;
        let first_assets_to_distribute = convert_to_assets(
            first_shares_to_distribute,
            share_price_num,
            share_price_denom,
        );

        // calculate the expected distribution amounts for the second allocation
        let second_amount = Uint256::from(second_allocation_amount);
        let lhs = second_amount * second_alloc_share_price_denom * price_scaling
            / second_alloc_share_price_num;
        let rhs = second_amount * share_price_denom * price_scaling / share_price_num;
        let shares_to_distribute_before_fees = Uint128::try_from(lhs - rhs).unwrap().u128();

        // calculate the distribution fees in truinj
        let second_fees =
            shares_to_distribute_before_fees * dist_fee as u128 / FEE_PRECISION as u128;

        // calculate shares for the distribution after fees
        let second_shares_to_distribute = shares_to_distribute_before_fees - second_fees;
        let second_assets_to_distribute = convert_to_assets(
            second_shares_to_distribute,
            share_price_num,
            share_price_denom,
        );

        // verify the truinj amounts required for the distribution equals the total fees
        let total_distribution_shares = first_shares_to_distribute + second_shares_to_distribute;
        let total_distribution_assets = first_assets_to_distribute + second_assets_to_distribute;
        let total_fees = first_fees + second_fees;

        // verify the distribution amounts
        assert_eq!(
            dist_amounts_res,
            GetDistributionAmountsResponse {
                truinj_amount: Uint128::from(total_distribution_shares),
                inj_amount: Uint128::from(total_distribution_assets),
                distribution_fee: Uint128::from(total_fees),
            },
        );
    }

    #[test]
    fn test_get_distribution_amounts_to_all_no_allocations() {
        let owner = "owner".into_bech32();
        let (app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner,
            "treasury".into_bech32(),
            0,
            0,
        );

        // get the amounts for a distribution to no recipients
        let dist_amounts_res: GetDistributionAmountsResponse = app
            .wrap()
            .query_wasm_smart(
                staker_addr,
                &QueryMsg::GetDistributionAmounts {
                    distributor: "other-distributor".into_bech32(),
                    recipient: None,
                },
            )
            .unwrap();

        // verify the distribution amounts are zero
        assert_eq!(
            dist_amounts_res,
            GetDistributionAmountsResponse {
                truinj_amount: Uint128::zero(),
                inj_amount: Uint128::zero(),
                distribution_fee: Uint128::zero(),
            },
        );
    }
}
