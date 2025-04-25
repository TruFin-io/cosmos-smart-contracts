pub mod helpers;

#[cfg(test)]
mod redelegation {
    use crate::helpers::{
        self, add_validator, assert_error, assert_event_with_attributes, get_delegation,
        get_total_rewards, get_total_staked, instantiate_staker_with_min_deposit_and_initial_stake,
        mint_inj, move_days_forward, query_truinj_balance, redelegate, stake,
        stake_to_specific_validator, whitelist_user,
    };

    use cosmwasm_std::{assert_approx_eq, Addr, Attribute, Uint128};
    use cw_multi_test::IntoBech32;
    use helpers::instantiate_staker;
    use injective_staker::ONE_INJ;

    #[test]
    fn test_redelegation() {
        let owner = "owner".into_bech32();
        let dst_validator_addr = "dst-validator".into_bech32();

        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                0,
                0,
            );

        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            dst_validator_addr.clone(),
        )
        .unwrap();

        // User1 stakes 100 INJ in the default validator //

        // mint INJ tokens to the 'user1' user
        let user1: Addr = "user1".into_bech32();
        let inj_to_mint_for_user1 = 100_000;
        mint_inj(&mut app, &user1, inj_to_mint_for_user1);
        whitelist_user(&mut app, &staker_addr, &owner, &user1);
        stake(&mut app, &user1, &staker_addr, inj_to_mint_for_user1).unwrap();

        // accumulate rewards
        move_days_forward(&mut app, 30);

        let total_rewards = get_total_rewards(&app, &staker_addr);
        assert_eq!(total_rewards.u128(), 402);

        // User2 stakes 300 INJ in the second validator //

        // mint INJ tokens to the 'user2' user
        let user2: Addr = "user2".into_bech32();
        let inj_to_mint_for_user2 = 1_000_000; // 300 INJ
        mint_inj(&mut app, &user2, inj_to_mint_for_user2);
        whitelist_user(&mut app, &staker_addr, &owner, &user2);
        stake_to_specific_validator(
            &mut app,
            &user2,
            &staker_addr,
            inj_to_mint_for_user2,
            &dst_validator_addr,
        )
        .unwrap();

        // ensure total_staked increased
        let total_staked = get_total_staked(&app, &staker_addr);
        assert_eq!(
            total_staked.u128(),
            inj_to_mint_for_user1 + inj_to_mint_for_user2
        );

        // accumulate rewards
        move_days_forward(&mut app, 30);

        let total_rewards = get_total_rewards(&app, &staker_addr);
        assert_eq!(total_rewards.u128(), 4832);

        // redelegate
        redelegate(
            &mut app,
            &owner,
            &staker_addr,
            &validator_addr,
            &dst_validator_addr,
            100_000,
        )
        .unwrap();

        // 805 INJ reward moved to the contract

        // after the redelegation, the total staked amount stays the same
        // but reward went to the contract so the validators would slightly decrease
        let total_rewards = get_total_rewards(&app, &staker_addr);
        assert_eq!(total_rewards.u128(), 4027);

        let total_staked = get_total_staked(&app, &staker_addr);
        assert_eq!(
            total_staked.u128(),
            inj_to_mint_for_user1 + inj_to_mint_for_user2
        );
    }

    #[test]
    fn test_redelegation_can_only_be_called_by_the_owner() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker(owner, "treasury".into_bech32());

        let user = "user".into_bech32();

        // redelegate
        let error_response = redelegate(
            &mut app,
            &user,
            &staker_addr,
            &"src-validator".into_bech32(),
            &"dst-validator".into_bech32(),
            100_000,
        )
        .unwrap_err();
        let error_source = error_response.source().unwrap();

        assert_eq!(
            error_source.to_string(),
            "Only the owner can call this method"
        );
    }

    #[test]
    fn test_redelegation_with_non_existent_src_validator_fails() {
        let owner = "owner".into_bech32();
        let src_validator_addr = "src-validator".into_bech32();
        let dst_validator_addr = "dst-validator".into_bech32();

        let (mut app, staker_addr, _) = instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            dst_validator_addr.clone(),
        )
        .unwrap();

        // redelegate
        let response = redelegate(
            &mut app,
            &owner,
            &staker_addr,
            &src_validator_addr,
            &dst_validator_addr,
            100_000,
        );

        assert_error(response, "Validator does not exist");
    }

    #[test]
    fn test_redelegation_with_non_existent_dst_validator_fails() {
        let owner = "owner".into_bech32();
        let src_validator_addr = "src-validator".into_bech32();
        let dst_validator_addr = "dst-validator".into_bech32();

        let (mut app, staker_addr, _) = instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            src_validator_addr.clone(),
        )
        .unwrap();

        // redelegate
        let response = redelegate(
            &mut app,
            &owner,
            &staker_addr,
            &src_validator_addr,
            &dst_validator_addr,
            100_000,
        );
        assert_error(response, "Validator does not exist");
    }

    #[test]
    fn test_redelegation_zero_assets_fails() {
        let owner = "owner".into_bech32();
        let src_validator_addr = "default-validator".into_bech32();
        let dst_validator_addr = "dst-validator".into_bech32();

        let (mut app, staker_addr, _) = instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            dst_validator_addr.clone(),
        )
        .unwrap();

        // redelegate
        let response = redelegate(
            &mut app,
            &owner,
            &staker_addr,
            &src_validator_addr,
            &dst_validator_addr,
            0,
        );
        assert_error(response, "Redelegate amount too low");
    }

    #[test]
    fn test_redelegation_more_than_total_staked_fails() {
        let owner = "owner".into_bech32();
        let dst_validator_addr = "dst-validator".into_bech32();

        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                ONE_INJ,
                0,
            );

        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            dst_validator_addr.clone(),
        )
        .unwrap();

        // mint INJ tokens to the 'anyone' user
        let anyone: Addr = "anyone".into_bech32();
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &staker_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &staker_addr, inj_to_mint).unwrap();

        // ensure user was minted TruINJ
        let sender_balance = query_truinj_balance(&app, &anyone, &staker_addr);
        assert_eq!(sender_balance, inj_to_mint);

        // ensure total_staked increased
        let total_staked = get_total_staked(&app, &staker_addr);
        assert_eq!(total_staked.u128(), inj_to_mint);

        // try to redelegate more than total_staked
        let response = redelegate(
            &mut app,
            &owner,
            &staker_addr,
            &validator_addr,
            &dst_validator_addr,
            100000000000000000000, // 100 INJ
        );
        assert_error(response, "Insufficient funds on validator");
    }

    #[test]
    fn test_redelegation_rewards_are_accruing_during_unbonding_period() {
        let owner = "owner".into_bech32();
        let dst_validator_addr = "dst-validator".into_bech32();

        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                0,
                0,
            );

        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            dst_validator_addr.clone(),
        )
        .unwrap();

        // mint INJ tokens to the 'alice' user
        let alice: Addr = "alice".into_bech32();
        let stake_amount = 100_000;
        mint_inj(&mut app, &alice, stake_amount);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // accrue rewards
        move_days_forward(&mut app, 30);

        let total_rewards = get_total_rewards(&app, &staker_addr);
        let expected_total_rewards = 402;
        assert_eq!(total_rewards.u128(), expected_total_rewards);

        // redelegate
        redelegate(
            &mut app,
            &owner,
            &staker_addr,
            &validator_addr,
            &dst_validator_addr,
            stake_amount,
        )
        .unwrap();

        // after redelegation, the rewards are sent to the contract and the validator gets empty
        let total_rewards_after_redelegate = get_total_rewards(&app, &staker_addr);
        assert_eq!(total_rewards_after_redelegate.u128(), 0);

        // accrue rewards
        move_days_forward(&mut app, 10);

        // after the redelegation, the total staked amount should stay the same and the rewards now accrue again
        let total_rewards = get_total_rewards(&app, &staker_addr);
        assert_approx_eq!(total_rewards.u128(), expected_total_rewards, "4");

        let total_staked = get_total_staked(&app, &staker_addr);
        assert_eq!(total_staked.u128(), stake_amount);
    }

    #[test]
    fn test_redelegation_emits_redelegated_event() {
        let owner = "owner".into_bech32();
        let dst_validator_addr = "dst-validator".into_bech32();

        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                0,
                0,
            );

        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            dst_validator_addr.clone(),
        )
        .unwrap();

        let amount = 100_000;

        // mint INJ tokens to the 'alice' user
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, amount);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);
        stake(&mut app, &alice, &staker_addr, amount).unwrap();

        // accrue rewards
        move_days_forward(&mut app, 1);

        // verify that staking rewards did accrue
        let rewards = get_total_rewards(&app, &staker_addr).u128();
        assert_eq!(rewards, 13);

        // ensure total_staked increased
        let total_staked = get_total_staked(&app, &staker_addr);
        assert_eq!(total_staked.u128(), amount);

        // accrue rewards
        move_days_forward(&mut app, 30);

        let total_rewards = get_total_rewards(&app, &staker_addr);
        let expected_total_rewards = 416;
        assert_eq!(total_rewards.u128(), expected_total_rewards);

        let delegation = get_delegation(&app, staker_addr.to_string(), &validator_addr);
        let src_validator_total_staked = delegation.amount.amount.u128();
        assert_eq!(src_validator_total_staked, amount);

        let amount_to_redelegate = 99_000u128;

        // redelegate
        let response = redelegate(
            &mut app,
            &owner,
            &staker_addr,
            &validator_addr,
            &dst_validator_addr,
            amount_to_redelegate,
        )
        .unwrap();

        let total_staked = get_total_staked(&app, &staker_addr);
        assert_eq!(total_staked.u128(), amount);

        assert_event_with_attributes(
            &response.events,
            "wasm-redelegated",
            vec![
                ("src_validator", validator_addr.to_string()).into(),
                ("dst_validator", dst_validator_addr.to_string()).into(),
                ("assets", Uint128::from(amount_to_redelegate)).into(),
            ],
            staker_addr.clone(),
        );

        let emitted_redelegate_event = response
            .events
            .iter()
            .find(|p: &&_| p.ty == "redelegate")
            .expect("Event not emitted");

        let expected_redelegate_attributes: Vec<Attribute> = vec![
            ("source_validator", validator_addr.to_string()).into(),
            ("destination_validator", dst_validator_addr.to_string()).into(),
            ("amount", format!("{}inj", amount_to_redelegate)).into(),
        ];
        assert_eq!(
            emitted_redelegate_event.attributes,
            expected_redelegate_attributes
        );
    }
}
