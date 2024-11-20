pub mod helpers;

#[cfg(test)]
mod unstake {

    use cosmwasm_std::{assert_approx_eq, to_json_binary, Addr, Uint128, WasmMsg};
    use cw_multi_test::{Executor, IntoBech32};
    use helpers::{mint_inj, stake};
    use injective_staker::{
        msg::ExecuteMsg, state::GetValueTrait, FEE_PRECISION, ONE_INJ, SHARE_PRICE_SCALING_FACTOR,
        UNBONDING_PERIOD,
    };

    use crate::helpers::{
        self, add_validator, assert_error, assert_event_with_attributes, clear_whitelist_status,
        get_claimable_assets, get_max_withdraw, get_share_price, get_total_rewards,
        get_total_staked, instantiate_staker_with_min_deposit,
        instantiate_staker_with_min_deposit_and_initial_stake, move_days_forward, pause,
        query_inj_balance, query_truinj_balance, query_truinj_supply, set_fee,
        stake_to_specific_validator, stake_when_rewards_accrued, unstake,
        unstake_when_rewards_accrue, whitelist_user,
    };

    #[test]
    fn test_unstake_partial_amount() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                0,
                1_000_000,
            );

        // mint some INJ tokens to alice
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // alice stakes
        let stake_amount = 100_000;
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        let pre_user_shares = query_truinj_balance(&app, &alice, &staker_addr);

        // accrue rewards
        move_days_forward(&mut app, 1);

        // verify that staking rewards did accrue
        let pre_rewards = get_total_rewards(&app, &staker_addr).u128();
        assert!(pre_rewards > 0);

        // verify there are no assets in the staker apart from the reserve amount
        let pre_staker_asset_balance = query_inj_balance(&app, &staker_addr);
        assert_eq!(pre_staker_asset_balance, 1);

        let pre_total_staked = get_total_staked(&app, &staker_addr).u128();
        let pre_share_price = get_share_price(&app, &staker_addr);
        let pre_total_supply = query_truinj_supply(&app, &staker_addr);

        // alice unstakes a partial amount
        let unstake_amount = 40_000;
        let unstake_res = unstake_when_rewards_accrue(
            &mut app,
            &alice,
            &staker_addr,
            unstake_amount,
            &validator_addr,
        );

        // verify the correct amount of assets was unstaked
        let total_staked = get_total_staked(&app, &staker_addr).u128();
        let unstaked_stake = pre_total_staked - total_staked;
        assert_eq!(unstaked_stake, unstake_amount);

        // verify the share price is the same except for a rounding error
        let share_price = get_share_price(&app, &staker_addr);
        assert_approx_eq!(share_price, pre_share_price, "0.000001");

        // verify the correct amount of shares were burned
        let user_shares = query_truinj_balance(&app, &alice, &staker_addr);
        let shares_burned = pre_user_shares - user_shares;
        let expected_shares_burned = unstake_amount * SHARE_PRICE_SCALING_FACTOR / share_price;
        assert_eq!(shares_burned, expected_shares_burned);

        // verify a record for the user claimable assets was added
        let claimable = get_claimable_assets(&app, &staker_addr, &alice);
        assert_eq!(claimable[0].amount.u128(), unstake_amount);

        // verify the truinj total supply was reduced
        let total_supply = query_truinj_supply(&app, &staker_addr);
        assert_eq!(total_supply, pre_total_supply - expected_shares_burned);

        // verify the assets in the staker is now the reward amount + reserve amount
        let staker_asset_balance = query_inj_balance(&app, &staker_addr);
        assert_eq!(staker_asset_balance, pre_rewards + 1);

        // verify that staking rewards are now 0
        let rewards = get_total_rewards(&app, &staker_addr).u128();
        assert_eq!(rewards, 0);

        let expiration = UNBONDING_PERIOD.after(&app.block_info());

        // verify the unstaked event was emitted
        assert_event_with_attributes(
            &unstake_res.unwrap().events,
            "wasm-unstaked",
            vec![
                ("user", alice.as_str()).into(),
                ("amount", "40000").into(),
                ("validator_addr", validator_addr.to_string()).into(),
                ("user_balance", user_shares.to_string()).into(),
                ("user_shares_burned", shares_burned.to_string()).into(),
                ("treasury_shares_minted", Uint128::zero()).into(),
                ("treasury_balance", Uint128::zero()).into(),
                ("total_staked", total_staked.to_string()).into(),
                ("total_supply", total_supply.to_string()).into(),
                ("expires_at", expiration.get_value().to_string()).into(),
            ],
            staker_addr,
        );
    }

    #[test]
    fn test_unstake_max_withdraw() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                0,
                0,
            );

        // mint some INJ tokens to alice
        let alice: Addr = "alice".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount * 2);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // alice stakes
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // accrue rewards
        move_days_forward(&mut app, 1);

        // verify that staking rewards did accrue
        let pre_rewards = get_total_rewards(&app, &staker_addr).u128();
        assert!(pre_rewards > 0);

        // verify there are no assets in the staker apart from the reserve amount
        let pre_staker_asset_balance = query_inj_balance(&app, &staker_addr);
        assert_eq!(pre_staker_asset_balance, 1);

        let pre_total_staked = get_total_staked(&app, &staker_addr).u128();

        // alice unstakes the max withdraw amount, which is greater than total_staked amount
        let max_withdraw = get_max_withdraw(&app, &staker_addr, &alice);
        assert!(max_withdraw > pre_total_staked);
        unstake_when_rewards_accrue(
            &mut app,
            &alice,
            &staker_addr,
            max_withdraw,
            &validator_addr,
        )
        .unwrap();

        // verify the total staked is now 0
        let total_staked = get_total_staked(&app, &staker_addr).u128();
        assert_eq!(total_staked, 0);

        // verify the share price is now 1.0
        let share_price = get_share_price(&app, &staker_addr);
        assert_eq!(share_price, ONE_INJ);

        // verify all user shares were burned
        let user_shares = query_truinj_balance(&app, &alice, &staker_addr);
        assert_eq!(user_shares, 0);

        // verify a record for the user claimable assets was added
        let claimable = get_claimable_assets(&app, &staker_addr, &alice);
        assert_eq!(claimable[0].amount.u128(), max_withdraw);

        // verify the truinj total supply was reduced
        let total_supply = query_truinj_supply(&app, &staker_addr);
        assert_eq!(total_supply, 0);

        // verify the assets in the staker is now the reward amount + reserve amount
        let staker_asset_balance = query_inj_balance(&app, &staker_addr);
        assert_eq!(staker_asset_balance, pre_rewards + 1);

        // verify that staking rewards are now 0
        let rewards = get_total_rewards(&app, &staker_addr).u128();
        assert_eq!(rewards, 0);
    }

    #[test]
    fn test_unstaked_rewards_are_not_swept() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                0,
                0,
            );

        // mint some INJ tokens to alice and bob
        let alice: Addr = "alice".into_bech32();
        let bob: Addr = "bob".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount * 2);
        mint_inj(&mut app, &bob, stake_amount * 3);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        whitelist_user(&mut app, &staker_addr, &owner, &bob);

        // add a second validator
        let second_validator: Addr = "second_validator".into_bech32();
        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            second_validator.clone(),
        )
        .unwrap();

        // alice and bob stake to different validators
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();
        stake_to_specific_validator(
            &mut app,
            &bob,
            &staker_addr,
            stake_amount,
            &second_validator,
        )
        .unwrap();

        // accrue rewards
        move_days_forward(&mut app, 1);

        // verify that staking rewards did accrue
        let pre_rewards = get_total_rewards(&app, &staker_addr).u128();
        assert!(pre_rewards > 0);

        // verify there are no assets in the staker apart from the reserve amount
        let pre_staker_asset_balance = query_inj_balance(&app, &staker_addr);
        assert_eq!(pre_staker_asset_balance, 1);

        let pre_share_price: u128 = get_share_price(&app, &staker_addr);

        // alice unstakes the max withdraw amount
        let max_withdraw = get_max_withdraw(&app, &staker_addr, &alice);
        unstake_when_rewards_accrue(
            &mut app,
            &alice,
            &staker_addr,
            max_withdraw,
            &validator_addr,
        )
        .unwrap();

        // bob stakes again, moving his accrued rewards into the validator
        stake_when_rewards_accrued(
            &mut app,
            &bob,
            &staker_addr,
            stake_amount,
            &second_validator,
        )
        .unwrap();

        // verify the total staked is now bobs staked amount
        let total_staked = get_total_staked(&app, &staker_addr).u128();
        assert_eq!(total_staked, stake_amount * 2);

        // verify the share price is the same except for a rounding error
        let share_price = get_share_price(&app, &staker_addr);
        assert_approx_eq!(share_price, pre_share_price, "0.000001");

        // verify the assets in the staker is now the reward amounts + reserve amount
        let staker_asset_balance = query_inj_balance(&app, &staker_addr);
        assert_eq!(staker_asset_balance, pre_rewards + 1);

        // verify that staking rewards are now 0
        let rewards = get_total_rewards(&app, &staker_addr).u128();
        assert_eq!(rewards, 0);

        // stake so that rewards are swept
        stake(&mut app, &bob, &staker_addr, stake_amount).unwrap();

        // verify that the unstaked rewards are not swept
        let unstaked_rewards = pre_rewards / 2; // alice had half the rewards
        let staker_asset_balance = query_inj_balance(&app, &staker_addr);
        assert_eq!(staker_asset_balance, unstaked_rewards + 1); // must add the reserve amount
    }

    #[test]
    pub fn test_unstake_multiple_unstakes() {
        let owner = "owner".into_bech32();
        let min_deposit = 1000;
        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                min_deposit,
                1_000_000,
            );

        // mint some INJ tokens to alice
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // alice stakes
        let stake_amount = 100_000;
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // rewards accrue
        move_days_forward(&mut app, 10);
        unstake_when_rewards_accrue(&mut app, &alice, &staker_addr, 50_000, &validator_addr)
            .unwrap();

        move_days_forward(&mut app, 10);
        unstake_when_rewards_accrue(&mut app, &alice, &staker_addr, 30_000, &validator_addr)
            .unwrap();

        // unstake remaining amount
        move_days_forward(&mut app, 10);
        let max_withdraw = get_max_withdraw(&app, &staker_addr, &alice);
        unstake_when_rewards_accrue(
            &mut app,
            &alice,
            &staker_addr,
            max_withdraw,
            &validator_addr,
        )
        .unwrap();

        // verify all user shares were burned
        let user_shares = query_truinj_balance(&app, &alice, &staker_addr);
        assert_eq!(user_shares, 0);

        // verify the claimable assets match the max withdrawable amounts
        let claimable = get_claimable_assets(&app, &staker_addr, &alice);
        assert_eq!(claimable.len(), 3);
        assert_eq!(claimable[0].amount.u128(), 50_000);
        assert_eq!(claimable[1].amount.u128(), 30_000);
        assert_eq!(claimable[2].amount.u128(), max_withdraw);
    }

    #[test]
    pub fn test_unstake_below_sweep_level_unstakes_remaining_stake() {
        let owner = "owner".into_bech32();
        let min_deposit = 10_000;
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            min_deposit,
            10_000,
        );

        // mint some INJ tokens to alice
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // alice stakes
        let stake_amount = 50_000;
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // rewards accrue
        move_days_forward(&mut app, 1);

        // unstake an amount of assets sufficient to bring the remaining stake below the sweep level (the min deposit)
        let max_withdraw = get_max_withdraw(&app, &staker_addr, &alice);
        let unstake_amount = max_withdraw - min_deposit + 1;
        let _ = unstake(&mut app, &alice, &staker_addr, unstake_amount);

        // verify all user shares were burned
        let user_shares = query_truinj_balance(&app, &alice, &staker_addr);
        assert_eq!(user_shares, 0);

        // verify the claimable assets matches the max withdraw amount
        let claimable = get_claimable_assets(&app, &staker_addr, &alice);
        assert_eq!(claimable[0].amount.u128(), max_withdraw);
    }

    #[test]
    fn test_unstake_with_fees() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                treasury.clone(),
                0,
                1_000_000,
            );

        // set 5% fee
        let fee = 500;
        set_fee(&mut app, &staker_addr, &owner, fee);

        // whitelist user with some tokens
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes
        let stake_amount = 100_000;
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        let pre_user_shares = query_truinj_balance(&app, &alice, &staker_addr);

        // rewards accrue
        move_days_forward(&mut app, 30);

        let pre_share_price = get_share_price(&app, &staker_addr);
        let pre_total_supply = query_truinj_supply(&app, &staker_addr);

        // verify the treasury truinj balance is zero before unstaking
        let treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);
        assert_eq!(treasury_truinj_balance, 0);

        // user unstakes a partial amount
        let unstake_amount = 40_000;
        let unstake_res = unstake(&mut app, &alice, &staker_addr, unstake_amount);

        // verify the correct amount of shares were burned
        let user_shares = query_truinj_balance(&app, &alice, &staker_addr);
        let shares_burned = pre_user_shares - user_shares;
        let expected_shares_burned = unstake_amount * SHARE_PRICE_SCALING_FACTOR / pre_share_price;
        assert_eq!(shares_burned, expected_shares_burned);

        // expect treasury fees to be 5% of the staking rewards
        let total_rewards = get_total_rewards(&app, &staker_addr);
        let expected_treasury_fees =
            total_rewards.u128() * fee as u128 * SHARE_PRICE_SCALING_FACTOR
                / pre_share_price
                / FEE_PRECISION as u128;

        let total_supply = query_truinj_supply(&app, &staker_addr);
        assert_eq!(
            total_supply,
            pre_total_supply + expected_treasury_fees - expected_shares_burned
        );

        let total_staked = get_total_staked(&app, &staker_addr).u128();

        let expiration = UNBONDING_PERIOD.after(&app.block_info());

        // verify the unstaked event was emitted
        assert_event_with_attributes(
            &unstake_res.unwrap().events,
            "wasm-unstaked",
            vec![
                ("user", alice.as_str()).into(),
                ("amount", "40000").into(),
                ("validator_addr", validator_addr.to_string()).into(),
                ("user_balance", user_shares.to_string()).into(),
                ("user_shares_burned", shares_burned.to_string()).into(),
                (
                    "treasury_shares_minted",
                    Uint128::from(expected_treasury_fees),
                )
                    .into(),
                ("treasury_balance", Uint128::from(expected_treasury_fees)).into(),
                ("total_staked", total_staked.to_string()).into(),
                ("total_supply", total_supply.to_string()).into(),
                ("expires_at", expiration.get_value().to_string()).into(),
            ],
            staker_addr.clone(),
        );

        // verify the treasury received the expected fees
        let treasury_tryinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);
        assert_eq!(treasury_tryinj_balance, expected_treasury_fees);
    }

    #[test]
    pub fn test_unstake_when_user_not_whitelisted_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        // whitelist user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes
        stake(&mut app, &alice, &staker_addr, 10_000).unwrap();

        // remove user from whitelist
        clear_whitelist_status(&mut app, &staker_addr, &owner, &alice);

        // verify that unstaking fails with the expected error
        let unstake_res = unstake(&mut app, &alice, &staker_addr, 5_000);
        assert_error(unstake_res, "User not whitelisted")
    }

    #[test]
    pub fn test_unstake_when_contract_is_paused_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        // whitelist user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes
        stake(&mut app, &alice, &staker_addr, 10_000).unwrap();

        // pause the contact
        pause(&mut app, &staker_addr, &owner);

        // verify that unstaking fails with the expected error
        let unstake_res = unstake(&mut app, &alice, &staker_addr, 5_000);
        assert_error(unstake_res, "Contract is paused");
    }

    #[test]
    pub fn test_unstake_zero_assets_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        // whitelist user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes
        stake(&mut app, &alice, &staker_addr, 10_000).unwrap();

        let pre_total_staked = get_total_staked(&app, &staker_addr).u128();
        let pre_user_shares = query_truinj_balance(&app, &alice, &staker_addr);

        // user unstakes zero assets
        let unstake_res = unstake(&mut app, &alice, &staker_addr, 0);

        // verify that unstaking fails with the expected error
        assert_error(unstake_res, "Unstake amount too low");

        // verify that the total staked and user shares remain unchanged
        let total_staked = get_total_staked(&app, &staker_addr).u128();
        assert_eq!(total_staked, pre_total_staked);

        let user_shares = query_truinj_balance(&app, &alice, &staker_addr);
        assert_eq!(user_shares, pre_user_shares);
    }

    #[test]
    pub fn test_unstake_zero_shares_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            10_000,
        );

        // whitelist user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes
        stake(&mut app, &alice, &staker_addr, 10_000).unwrap();

        // rewards accrue
        move_days_forward(&mut app, 365);

        // user unstakes 1inj
        let unstake_res = unstake(&mut app, &alice, &staker_addr, 1);
        assert!(unstake_res.is_err());

        // verify that unstaking fails with the expected error
        assert_error(unstake_res, "Shares amount too low");
    }

    #[test]
    pub fn test_unstake_more_than_max_withdraw_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            1_000_000,
        );

        // whitelist user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes
        stake(&mut app, &alice, &staker_addr, 10_000).unwrap();

        // rewards accrue
        move_days_forward(&mut app, 30);

        let pre_total_staked = get_total_staked(&app, &staker_addr).u128();
        let pre_user_shares = query_truinj_balance(&app, &alice, &staker_addr);

        // user unstakes more than max_withdraw
        let max_withdraw = get_max_withdraw(&app, &staker_addr, &alice);
        let unstake_amount = max_withdraw + 1;
        let unstake_res = unstake(&mut app, &alice, &staker_addr, unstake_amount);

        // verify that unstaking fails with the expected error
        assert_error(unstake_res, "Insufficient TruINJ balance");

        // verify that the total staked and user shares remain unchanged
        let total_staked = get_total_staked(&app, &staker_addr).u128();
        assert_eq!(total_staked, pre_total_staked);

        let user_shares = query_truinj_balance(&app, &alice, &staker_addr);
        assert_eq!(user_shares, pre_user_shares);
    }

    #[test]
    pub fn test_unstake_from_specific_validator() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            1_000_000,
        );

        // add a second validator
        let second_validator: Addr = "second-validator".into_bech32();
        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            second_validator.clone(),
        )
        .unwrap();

        // mint tokens and whitelist a user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes to the second validator
        let stake_amount = 100_000;
        stake_to_specific_validator(
            &mut app,
            &alice,
            &staker_addr,
            stake_amount,
            &second_validator.clone(),
        )
        .unwrap();

        let pre_user_shares = query_truinj_balance(&app, &alice, &staker_addr);

        // rewards accrue
        move_days_forward(&mut app, 1);

        let pre_total_staked = get_total_staked(&app, &staker_addr).u128();
        let pre_share_price = get_share_price(&app, &staker_addr);
        let pre_total_supply = query_truinj_supply(&app, &staker_addr);

        // user unstakes a partial amount from the second validator
        let unstake_amount = 40_000;
        let unstake_res = unstake_when_rewards_accrue(
            &mut app,
            &alice,
            &staker_addr,
            unstake_amount,
            &second_validator,
        );

        assert!(unstake_res.is_ok());

        // verify the correct amount of assets was unstaked
        let total_staked = get_total_staked(&app, &staker_addr).u128();
        let unstaked_stake = pre_total_staked - total_staked;
        assert_eq!(unstaked_stake, unstake_amount);

        // verify the share price is the same except for a rounding error
        let share_price = get_share_price(&app, &staker_addr);
        assert_approx_eq!(share_price, pre_share_price, "0.000001");

        // verify the correct amount of shares were burned
        let user_shares = query_truinj_balance(&app, &alice, &staker_addr);
        let shares_burned = pre_user_shares - user_shares;
        let expected_shares_burned = unstake_amount * SHARE_PRICE_SCALING_FACTOR / share_price;
        assert_eq!(shares_burned, expected_shares_burned);

        // verify a record for the user claimable assets was added
        let claimable = get_claimable_assets(&app, &staker_addr, &alice);
        assert_eq!(claimable[0].amount.u128(), unstake_amount);

        // verify the truinj total supply was redutced
        let total_supply = query_truinj_supply(&app, &staker_addr);
        assert_eq!(total_supply, pre_total_supply - expected_shares_burned);

        let expiration = UNBONDING_PERIOD.after(&app.block_info());
        // verify the unstaked event was emitted
        assert_event_with_attributes(
            &unstake_res.unwrap().events,
            "wasm-unstaked",
            vec![
                ("user", alice.as_str()).into(),
                ("amount", "40000").into(),
                ("validator_addr", second_validator.to_string()).into(),
                ("user_balance", user_shares.to_string()).into(),
                ("user_shares_burned", shares_burned.to_string()).into(),
                ("treasury_shares_minted", Uint128::zero()).into(),
                ("treasury_balance", Uint128::zero()).into(),
                ("total_staked", total_staked.to_string()).into(),
                ("total_supply", total_supply.to_string()).into(),
                ("expires_at", expiration.get_value().to_string()).into(),
            ],
            staker_addr,
        );
    }

    #[test]
    pub fn test_unstake_from_specific_validator_when_contract_is_paused_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            100_000,
        );

        // add a second validator
        let second_validator: Addr = "second-validator".into_bech32();
        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            second_validator.clone(),
        )
        .unwrap();

        // whitelist user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes
        stake_to_specific_validator(&mut app, &alice, &staker_addr, 10_000, &second_validator)
            .unwrap();

        // pause the contact
        pause(&mut app, &staker_addr, &owner);

        // execute unstake from specific validator
        let unstake_res = app.execute(
            alice.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::UnstakeFromSpecificValidator {
                    validator_addr: second_validator.to_string(),
                    amount: Uint128::from(10_000u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify that unstaking from the specific validator fails with the expected error
        assert!(unstake_res.is_err());
        assert_error(unstake_res, "Contract is paused");
    }

    #[test]
    pub fn test_unstake_from_specific_validator_when_user_not_whitelisted_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            100_000,
        );

        // add a second validator
        let second_validator: Addr = "second-validator".into_bech32();
        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            second_validator.clone(),
        )
        .unwrap();

        // whitelist user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes
        stake_to_specific_validator(&mut app, &alice, &staker_addr, 10_000, &second_validator)
            .unwrap();

        // remove user from whitelist
        clear_whitelist_status(&mut app, &staker_addr, &owner, &alice);

        // execute unstake from specific validator
        let unstake_res = app.execute(
            alice.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::UnstakeFromSpecificValidator {
                    validator_addr: second_validator.to_string(),
                    amount: Uint128::from(10_000u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify that unstaking from the specific validator fails with the expected error
        assert!(unstake_res.is_err());
        assert_error(unstake_res, "User not whitelisted")
    }

    #[test]
    pub fn test_unstake_from_specific_validator_more_than_staked_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            1_000_000,
        );

        // add a second validator
        let second_validator: Addr = "second-validator".into_bech32();
        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            second_validator.clone(),
        )
        .unwrap();

        // mint tokens and whitelist a user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes to the default validator
        let stake_amount = 100_000u128;
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // user tries to unstake from the second validator more than is available
        let unstake_amount = 1_000u128;
        let unstake_res = app.execute(
            alice.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::UnstakeFromSpecificValidator {
                    validator_addr: second_validator.to_string(),
                    amount: unstake_amount.into(),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify that unstaking fails with the expected error
        assert!(unstake_res.is_err());
        assert_error(unstake_res, "Insufficient funds on validator")
    }

    #[test]
    pub fn test_unstake_from_specific_validator_with_nonexistent_address_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            1_000_000,
        );

        // mint tokens and whitelist a user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 100_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes to the default validator
        let staked_amount = 100_000u128;
        stake(&mut app, &alice, &staker_addr, staked_amount).unwrap();

        // user tries to unstake from a non existing validator
        let validator = "nonexistent".into_bech32();
        let unstake_res = app.execute(
            alice.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::UnstakeFromSpecificValidator {
                    validator_addr: validator.to_string(),
                    amount: Uint128::from(1000u128),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify that unstaking fails with the expected error
        assert!(unstake_res.is_err());
        assert_error(unstake_res, "Validator does not exist")
    }

    #[test]
    fn test_unstake_max_withdraw_with_multiple_users() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                0,
                0,
            );

        let users = [
            "user0".into_bech32(),
            "user1".into_bech32(),
            "user2".into_bech32(),
            "user3".into_bech32(),
            "user4".into_bech32(),
            "user5".into_bech32(),
            "user6".into_bech32(),
        ];

        // all users stake some inj
        let stake_amount = 100;
        for user in users.iter() {
            mint_inj(&mut app, user, stake_amount);
            whitelist_user(&mut app, &staker_addr, &owner, user);
            stake(&mut app, user, &staker_addr, stake_amount).unwrap();
        }

        // rewards accrue
        move_days_forward(&mut app, 2000);

        // all users unstake their max_withdraw
        // when the last user unstakes, there is nothing left on the validator as all the rewards have already been withdrawn into the staker
        let max_withdraw = get_max_withdraw(&app, &staker_addr, &users[0]);
        unstake_when_rewards_accrue(
            &mut app,
            &users[0],
            &staker_addr,
            max_withdraw,
            &validator_addr,
        )
        .unwrap();

        for user in users[1..].iter() {
            let max_withdraw = get_max_withdraw(&app, &staker_addr, user);
            unstake(&mut app, user, &staker_addr, max_withdraw).unwrap();
        }

        // verify all users max_withdraw is now zero
        for user in users.iter() {
            assert_eq!(get_max_withdraw(&app, &staker_addr, user), 0);
        }

        // verify the total staked and total rewards on the validator are now zero
        assert_eq!(get_total_staked(&app, &staker_addr).u128(), 0);
        assert_eq!(get_total_rewards(&app, &staker_addr).u128(), 0);
    }
}
