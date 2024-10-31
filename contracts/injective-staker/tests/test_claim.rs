pub mod helpers;

#[cfg(test)]
mod claim {

    use cosmwasm_std::{Addr, Decimal};
    use cw_multi_test::{IntoBech32, StakingSudo};
    use helpers::{mint_inj, stake};

    use crate::helpers::{
        self, assert_error, assert_event_with_attributes, claim, get_claimable_assets,
        get_max_withdraw, get_total_staked, instantiate_staker_with_min_deposit,
        instantiate_staker_with_min_deposit_and_initial_stake, move_days_forward, pause,
        query_inj_balance, unstake, unstake_when_rewards_accrue, whitelist_user,
    };

    #[test]
    fn test_claim() {
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

        // accrue rewards
        move_days_forward(&mut app, 1);

        // alice unstakes a partial amount
        let unstake_amount = 40_000;
        unstake_when_rewards_accrue(
            &mut app,
            &alice,
            &staker_addr,
            unstake_amount,
            &validator_addr,
        )
        .unwrap();

        move_days_forward(&mut app, 21);

        let pre_balance = query_inj_balance(&app, &alice);

        let claim_res = claim(&mut app, &alice, &staker_addr).unwrap();

        let post_balance = query_inj_balance(&app, &alice);

        assert!(post_balance == pre_balance + unstake_amount);

        // verify the withdraw event was emitted
        assert_event_with_attributes(
            &claim_res.events,
            "wasm-claimed",
            vec![("user", alice.as_str()).into(), ("amount", "40000").into()],
            staker_addr,
        );
    }

    #[test]
    fn test_claim_claims_all_available_unlocks() {
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

        // accrue rewards
        move_days_forward(&mut app, 1);

        // alice unstakes several times
        unstake_when_rewards_accrue(&mut app, &alice, &staker_addr, 40000, &validator_addr)
            .unwrap();
        unstake(&mut app, &alice, &staker_addr, 20000).unwrap();
        unstake(&mut app, &alice, &staker_addr, 15000).unwrap();
        unstake(&mut app, &alice, &staker_addr, 25000).unwrap();

        move_days_forward(&mut app, 21);

        let pre_balance = query_inj_balance(&app, &alice);

        claim(&mut app, &alice, &staker_addr).unwrap();

        let post_balance = query_inj_balance(&app, &alice);

        assert!(post_balance == pre_balance + stake_amount);

        let claimable = get_claimable_assets(&app, &staker_addr, &alice);
        assert!(claimable.is_empty());
    }

    #[test]
    fn test_claim_fails_when_not_ready() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
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

        // accrue rewards
        move_days_forward(&mut app, 1);

        // alice unstakes a partial amount
        let unstake_amount = 40_000;
        unstake(&mut app, &alice, &staker_addr, unstake_amount).unwrap();

        move_days_forward(&mut app, 20);

        let pre_balance = query_inj_balance(&app, &alice);

        let claim_res = claim(&mut app, &alice, &staker_addr);

        assert_error(claim_res, "No withdrawals to claim");

        let post_balance = query_inj_balance(&app, &alice);

        assert!(post_balance == pre_balance);
    }

    #[test]
    fn test_claim_fails_when_not_whitelisted() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner,
            "treasury".into_bech32(),
            0,
            1_000_000,
        );

        let alice: Addr = "alice".into_bech32();

        let claim_res = claim(&mut app, &alice, &staker_addr);

        assert_error(claim_res, "User not whitelisted");
    }

    #[test]
    fn test_claim_fails_when_contract_paused() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            1_000_000,
        );

        let alice: Addr = "alice".into_bech32();

        pause(&mut app, &staker_addr, &owner);
        let claim_res = claim(&mut app, &alice, &staker_addr);

        assert_error(claim_res, "Contract is paused");
    }

    #[test]
    fn test_claim_fails_when_user_has_no_claims() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            1_000_000,
        );

        let alice: Addr = "alice".into_bech32();
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        let claim_res = claim(&mut app, &alice, &staker_addr);

        assert_error(claim_res, "No withdrawals to claim");
    }

    #[test]
    fn test_claim_slash() {
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
        mint_inj(&mut app, &alice, 100_000_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // alice stakes
        let stake_amount = 100_000_000;
        stake(&mut app, &alice, &staker_addr, stake_amount).unwrap();

        // accrue rewards
        move_days_forward(&mut app, 1);

        // alice unstakes a partial amount
        let unstake_amount = 400_000;
        unstake_when_rewards_accrue(
            &mut app,
            &alice,
            &staker_addr,
            unstake_amount,
            &validator_addr,
        )
        .unwrap();

        move_days_forward(&mut app, 5);

        // Slash the validator by 50%
        app.sudo(cw_multi_test::SudoMsg::Staking(StakingSudo::Slash {
            validator: validator_addr.to_string(),
            percentage: Decimal::percent(50),
        }))
        .unwrap();

        move_days_forward(&mut app, 21);

        // claiming will now fail as the contract only received  half the unstaked amount
        let claim_res = claim(&mut app, &alice, &staker_addr);

        assert_error(claim_res, "Insufficient funds on staker");
    }

    #[test]
    fn test_claim_with_slash_does_not_allow_users_to_claim_full_amount_until_staker_has_funds() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit_and_initial_stake(
                owner.clone(),
                "treasury".into_bech32(),
                0,
                1_000_000,
            );

        let users = [
            "user0".into_bech32(),
            "user1".into_bech32(),
            "user2".into_bech32(),
            "user3".into_bech32(),
            "user4".into_bech32(),
        ];

        let stake_amount = 1000;
        for user in users.clone() {
            mint_inj(&mut app, &user, stake_amount);
            whitelist_user(&mut app, &staker_addr, &owner, &user);
            stake(&mut app, &user, &staker_addr, stake_amount).unwrap();
        }

        // accrue rewards
        move_days_forward(&mut app, 10);

        // all users unstake a different amount: 200, 400, 600, 800, 1000
        // for a total of 3000
        for (idx, user) in users.iter().enumerate() {
            let unstake_amount = 200 * (1 + idx as u128);
            unstake_when_rewards_accrue(
                &mut app,
                user,
                &staker_addr,
                unstake_amount,
                &validator_addr,
            )
            .unwrap();

            move_days_forward(&mut app, 1);
        }

        // slash the validator by 50% while the unbondings are still in progress
        // so that the staker will receive 1500 inj less than expected
        app.sudo(cw_multi_test::SudoMsg::Staking(StakingSudo::Slash {
            validator: validator_addr.to_string(),
            percentage: Decimal::percent(50),
        }))
        .unwrap();

        // wait untill all unbondings complete
        move_days_forward(&mut app, 21);

        // all users try to claim but only the first 3 should receive their funds
        // for a total of 200 + 400 + 600 = 1200
        for (idx, user) in users.iter().enumerate() {
            let claim_res = claim(&mut app, user, &staker_addr);
            let post_balance = query_inj_balance(&app, user);

            // verify that the first 3 users received their funds,
            // and the last 2 users didn't because the staker did not have enough funds
            if idx < 3 {
                assert!(claim_res.is_ok());
                let expected_amount = 200 * (1 + idx as u128);
                assert_eq!(post_balance, expected_amount);
            } else {
                assert!(claim_res.is_err());
                assert_error(claim_res, "Insufficient funds on staker");
            }
        }

        // mint the missing amount of inj to the staker
        mint_inj(&mut app, &staker_addr, 1500);

        // verify user3 and user4 can claim again and receive their funds
        let remaining_users = users[3..5].to_vec();
        for (idx, user) in remaining_users.iter().enumerate() {
            let claim_res = claim(&mut app, user, &staker_addr);
            let post_balance = query_inj_balance(&app, user);
            assert!(claim_res.is_ok());
            let expected_amount = 600 + 200 * (1 + idx as u128);
            assert_eq!(post_balance, expected_amount);
        }
    }

    #[test]
    fn test_claim_with_slash_when_no_unbondings_in_progress() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let users = [
            "user0".into_bech32(),
            "user1".into_bech32(),
            "user2".into_bech32(),
            "user3".into_bech32(),
        ];

        // all users stake 1000inj
        let stake_amount = 10_000;
        for user in users.clone() {
            mint_inj(&mut app, &user, stake_amount);
            whitelist_user(&mut app, &staker_addr, &owner, &user);
            stake(&mut app, &user, &staker_addr, stake_amount).unwrap();
        }

        // rewards accrue
        move_days_forward(&mut app, 10);

        // to keep track of the the unstaked amounts
        let mut unstaked_amounts: Vec<u128> = Vec::new();

        // the first 2 users unstake their max_withdraw
        let first_users = users[0..2].to_vec();
        for user in first_users.iter() {
            let max_withdraw = get_max_withdraw(&app, &staker_addr, user);
            assert!(max_withdraw > stake_amount);
            unstaked_amounts.push(max_withdraw);

            unstake_when_rewards_accrue(
                &mut app,
                user,
                &staker_addr,
                max_withdraw,
                &validator_addr,
            )
            .unwrap();

            move_days_forward(&mut app, 1);
        }

        // wait their unbondings complete
        move_days_forward(&mut app, 21);

        // first users claim, so they will receive their full amount
        for user in first_users.iter() {
            let claim_res = claim(&mut app, user, &staker_addr);
            assert!(claim_res.is_ok());
        }

        // slash the validator by 50% when no unbondings are in progress
        app.sudo(cw_multi_test::SudoMsg::Staking(StakingSudo::Slash {
            validator: validator_addr.to_string(),
            percentage: Decimal::percent(50),
        }))
        .unwrap();

        // the remaining users unstake their max_withdraw which has been slashed
        let second_users = users[2..4].to_vec();
        for user in second_users.iter() {
            let max_withdraw = get_max_withdraw(&app, &staker_addr, user);
            // the max withdraw should be less than the amount staked due to slashing
            assert!(max_withdraw < stake_amount);
            unstaked_amounts.push(max_withdraw);

            unstake_when_rewards_accrue(
                &mut app,
                user,
                &staker_addr,
                max_withdraw,
                &validator_addr,
            )
            .unwrap();
            move_days_forward(&mut app, 1);
        }

        // wait untill all unbondings complete
        move_days_forward(&mut app, 21);

        // second users claim
        for user in second_users.iter() {
            let claim_res = claim(&mut app, user, &staker_addr);
            assert!(claim_res.is_ok());
        }

        // verify all users received their unstaked amount
        for (idx, user) in users.iter().enumerate() {
            assert_eq!(query_inj_balance(&app, user), unstaked_amounts[idx]);
        }

        // verify the staker and validator have been completely drained except for the reserve amount
        assert_eq!(query_inj_balance(&app, &staker_addr), 1);
        assert_eq!(get_total_staked(&app, &staker_addr).u128(), 0);
    }
}
