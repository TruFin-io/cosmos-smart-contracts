pub mod helpers;

#[cfg(test)]
mod stake {

    use crate::helpers::{
        self, add_validator, assert_error, assert_event_with_attributes, disable_validator,
        get_share_price, get_share_price_num_denom, get_total_rewards, get_total_staked,
        instantiate_staker, instantiate_staker_with_min_deposit_and_initial_stake,
        move_days_forward, pause, query_truinj_balance, query_truinj_supply, set_fee,
        stake_to_specific_validator, stake_when_rewards_accrued, whitelist_user,
    };
    use cosmwasm_std::{Addr, Decimal, Uint128, Uint256};
    use cw_multi_test::{IntoBech32, StakingSudo};
    use helpers::{mint_inj, stake};
    use injective_staker::{
        msg::{GetTotalAssetsResponse, QueryMsg},
        FEE_PRECISION, ONE_INJ, SHARE_PRICE_SCALING_FACTOR,
    };

    #[test]
    fn test_stake() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                ONE_INJ,
                0,
            );

        // mint INJ tokens to the 'anyone' user
        let anyone: Addr = "anyone".into_bech32();
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        let stake_res = stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        // ensure user was minted TruINJ
        let sender_balance = query_truinj_balance(&app, &anyone, &contract_addr);
        assert_eq!(sender_balance, inj_to_mint);

        // ensure total_staked increased
        let total_staked = get_total_staked(&app, &contract_addr);
        assert_eq!(total_staked.u128(), inj_to_mint);

        let event_attribute_staked = Uint128::from(inj_to_mint);
        let treasury_shares_minted = Uint128::zero();

        assert_event_with_attributes(
            &stake_res.events,
            "wasm-deposited",
            vec![
                ("user", anyone.to_string()).into(),
                ("validator_addr", validator_addr).into(),
                ("amount", event_attribute_staked).into(),
                ("contract_rewards", Uint128::zero()).into(),
                ("user_shares_minted", event_attribute_staked).into(),
                ("treasury_shares_minted", treasury_shares_minted).into(),
                ("treasury_balance", treasury_shares_minted).into(),
                ("total_staked", event_attribute_staked).into(),
                ("total_supply", event_attribute_staked).into(),
                ("share_price_num", Uint256::from(SHARE_PRICE_SCALING_FACTOR)).into(),
                ("share_price_denom", Uint256::one()).into(),
                ("user_balance", event_attribute_staked).into(),
            ],
            contract_addr,
        );
    }

    #[test]
    fn test_stake_after_slashing() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                0,
                0,
            );

        // mint INJ tokens to the 'anyone' user
        let anyone: Addr = "anyone".into_bech32();
        let inj_to_mint = 100000000000;
        mint_inj(&mut app, &anyone, inj_to_mint * 2);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        // Slash the validator by 50%
        app.sudo(cw_multi_test::SudoMsg::Staking(StakingSudo::Slash {
            validator: validator_addr.to_string(),
            percentage: Decimal::percent(50),
        }))
        .unwrap();

        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        // user will have been minted double the truinj due to the share price being cut in half
        let sender_balance = query_truinj_balance(&app, &anyone, &contract_addr);
        assert_eq!(sender_balance, inj_to_mint * 3);

        // total staked increased by the staked amount but half of the original stake was slashed
        let total_staked = get_total_staked(&app, &contract_addr);
        assert_eq!(total_staked.u128(), inj_to_mint / 2 + inj_to_mint);
    }

    #[test]
    fn test_stake_to_specific_validator() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            ONE_INJ,
            0,
        );

        let second_validator: Addr = "second_validator".into_bech32();
        add_validator(
            &mut app,
            owner.clone(),
            &contract_addr,
            second_validator.clone(),
        )
        .unwrap();

        // mint INJ tokens to the 'anyone' user
        let anyone: Addr = "anyone".into_bech32();
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        let stake_res = stake_to_specific_validator(
            &mut app,
            &anyone,
            &contract_addr,
            inj_to_mint,
            &second_validator,
        )
        .unwrap();

        // ensure user was minted TruINJ
        let sender_balance = query_truinj_balance(&app, &anyone, &contract_addr);
        assert_eq!(sender_balance, inj_to_mint);

        // ensure total_staked increased
        let total_staked = get_total_staked(&app, &contract_addr);
        assert_eq!(total_staked.u128(), inj_to_mint);

        let event_attribute_staked = Uint128::from(inj_to_mint);
        let treasury_shares_minted = Uint128::zero();
        assert_event_with_attributes(
            &stake_res.events,
            "wasm-deposited",
            vec![
                ("user", anyone.to_string()).into(),
                ("validator_addr", second_validator).into(),
                ("amount", event_attribute_staked).into(),
                ("contract_rewards", Uint128::zero()).into(),
                ("user_shares_minted", event_attribute_staked).into(),
                ("treasury_shares_minted", treasury_shares_minted).into(),
                ("treasury_balance", treasury_shares_minted).into(),
                ("total_staked", event_attribute_staked).into(),
                ("total_supply", event_attribute_staked).into(),
                ("share_price_num", Uint256::from(SHARE_PRICE_SCALING_FACTOR)).into(),
                ("share_price_denom", Uint256::one()).into(),
                ("user_balance", event_attribute_staked).into(),
            ],
            contract_addr,
        );
    }

    #[test]
    fn test_stake_twice() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake twice
        stake(&mut app, &anyone, &contract_addr, inj_to_mint / 2).unwrap();
        stake(&mut app, &anyone, &contract_addr, inj_to_mint / 2).unwrap();

        // ensure user was minted TruINJ
        let sender_balance = query_truinj_balance(&app, &anyone, &contract_addr);
        assert_eq!(sender_balance, inj_to_mint);

        // ensure total_staked increased
        let total_staked = get_total_staked(&app, &contract_addr);
        assert_eq!(total_staked.u128(), inj_to_mint);
    }

    #[test]
    fn test_stake_with_fees() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, contract_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                treasury.clone(),
                0,
                100_000,
            );

        // set 5% fee
        let fee = 500;
        set_fee(&mut app, &contract_addr, &owner, fee);

        // mint INJ tokens to the 'anyone' user
        let anyone: Addr = "anyone".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &anyone, stake_amount);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // accumulate rewards
        move_days_forward(&mut app, 30);

        // calculate expected user shares
        let share_price = get_share_price(&app, &contract_addr);
        let expected_user_shares = Uint128::from(stake_amount)
            * Uint128::from(SHARE_PRICE_SCALING_FACTOR)
            / Uint128::from(share_price);
        let (share_price_num, share_price_denom) = get_share_price_num_denom(&app, &contract_addr);

        // verify the reasury truinj balance is zero
        let treasury_truinj_balance = query_truinj_balance(&app, &treasury, &contract_addr);
        assert_eq!(treasury_truinj_balance, 0);

        // user stakes
        let stake_res = stake(&mut app, &anyone, &contract_addr, stake_amount).unwrap();

        let total_rewards: Uint128 = get_total_rewards(&app, &contract_addr);
        let share_price = get_share_price(&app, &contract_addr);

        // expect treasury fees to be 5% of the rewards
        let expected_treasury_fees =
            total_rewards.u128() * fee as u128 * SHARE_PRICE_SCALING_FACTOR
                / share_price
                / FEE_PRECISION as u128;

        let total_staked = get_total_staked(&app, &contract_addr);
        let user_shares_balance = query_truinj_balance(&app, &anyone, &contract_addr);

        let shares_suply = query_truinj_supply(&app, &contract_addr);

        // verify the deposited event was emitted
        assert_event_with_attributes(
            &stake_res.events,
            "wasm-deposited",
            vec![
                ("user", anyone.to_string()).into(),
                ("validator_addr", validator_addr).into(),
                ("amount", Uint128::from(stake_amount)).into(),
                ("contract_rewards", Uint128::zero()).into(),
                ("user_shares_minted", Uint128::from(user_shares_balance)).into(),
                (
                    "treasury_shares_minted",
                    Uint128::from(expected_treasury_fees),
                )
                    .into(),
                ("treasury_balance", Uint128::from(expected_treasury_fees)).into(),
                ("total_staked", total_staked).into(),
                ("total_supply", Uint128::from(shares_suply)).into(),
                ("share_price_num", share_price_num).into(),
                ("share_price_denom", share_price_denom).into(),
                ("user_balance", expected_user_shares).into(),
            ],
            contract_addr.clone(),
        );

        // verify the treasury received the expected fees
        let treasury_tryinj_balance = query_truinj_balance(&app, &treasury, &contract_addr);
        assert_eq!(treasury_tryinj_balance, expected_treasury_fees);
    }

    #[test]
    fn test_stake_with_non_whitelisted_user_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) = instantiate_staker(owner, "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // execute stake
        let stake_res = stake(&mut app, &anyone, &contract_addr, inj_to_mint / 2);
        assert_error(stake_res, "User not whitelisted");

        // ensure user was not minted TruINJ
        let sender_balance = query_truinj_balance(&app, &anyone, &contract_addr);
        assert_eq!(sender_balance, 0);

        // ensure total_staked did not increase
        let total_staked = get_total_staked(&app, &contract_addr);
        assert_eq!(total_staked.u128(), 0);
    }

    #[test]
    fn test_stake_to_specific_validator_with_non_whitelisted_user_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, validator_addr) =
            instantiate_staker(owner, "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // execute stake
        let stake_res = stake_to_specific_validator(
            &mut app,
            &anyone,
            &contract_addr,
            inj_to_mint / 2,
            &validator_addr,
        );
        assert_error(stake_res, "User not whitelisted");

        // ensure user was not minted TruINJ
        let sender_balance = query_truinj_balance(&app, &anyone, &contract_addr);
        assert_eq!(sender_balance, 0);

        // ensure total_staked did not increase
        let total_staked = get_total_staked(&app, &contract_addr);
        assert_eq!(total_staked.u128(), 0);
    }

    #[test]
    fn test_stake_to_disabled_pool_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, validator_addr) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // disable validator
        disable_validator(&mut app, owner.clone(), &contract_addr, validator_addr).unwrap();

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        let stake_res = stake(&mut app, &anyone, &contract_addr, inj_to_mint);
        assert_error(stake_res, "Validator is disabled");
    }

    #[test]
    fn test_stake_to_nonexistent_pool_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        let stake_res =
            stake_to_specific_validator(&mut app, &anyone, &contract_addr, inj_to_mint, &anyone);
        assert_error(stake_res, "Validator does not exist");
    }

    #[test]
    fn test_stake_sweeps_rewards() {
        let anyone = "anyone".into_bech32();
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                1000,
                0,
            );
        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 1000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint * 3);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        // accrue rewards
        move_days_forward(&mut app, 1);

        // stake again, mimicking rewards being sweeped into the contract
        stake_when_rewards_accrued(
            &mut app,
            &anyone,
            &contract_addr,
            inj_to_mint,
            &validator_addr,
        )
        .unwrap();

        let total_assets: GetTotalAssetsResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::GetTotalAssets {})
            .unwrap();

        assert!(total_assets.total_assets.u128() > 0);

        let pre_total_staked = get_total_staked(&app, &contract_addr);

        // now stake again
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        let post_total_assets: GetTotalAssetsResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::GetTotalAssets {})
            .unwrap();

        assert_eq!(post_total_assets.total_assets, Uint128::one());

        let post_total_staked = get_total_staked(&app, &contract_addr);
        assert!(
            post_total_staked + post_total_assets.total_assets
                == pre_total_staked + Uint128::from(inj_to_mint) + total_assets.total_assets
        );
    }

    #[test]
    fn test_stake_when_contract_paused_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // pause contract
        pause(&mut app, &contract_addr, &owner);

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        let stake_res = stake(&mut app, &anyone, &contract_addr, inj_to_mint);
        assert_error(stake_res, "Contract is paused");
    }

    #[test]
    fn test_stake_to_specific_validator_when_contract_paused_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // pause contract
        pause(&mut app, &contract_addr, &owner);

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        let stake_res =
            stake_to_specific_validator(&mut app, &anyone, &contract_addr, inj_to_mint, &anyone);
        assert_error(stake_res, "Contract is paused");
    }

    #[test]
    fn test_stake_below_min_deposit_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 100000000000000000; // 0.1 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        let stake_res = stake(&mut app, &anyone, &contract_addr, inj_to_mint);
        assert_error(stake_res, "Deposit amount is below the min deposit amount");
    }
}
