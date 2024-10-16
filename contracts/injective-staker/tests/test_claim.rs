pub mod helpers;

#[cfg(test)]
mod claim {

    use cosmwasm_std::Addr;
    use cw_multi_test::IntoBech32;
    use helpers::{mint_inj, stake};

    use crate::helpers::{
        self, assert_error, assert_event_with_attributes, claim, get_claimable_assets,
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
}
