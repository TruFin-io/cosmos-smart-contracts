pub mod helpers;

#[cfg(test)]
mod allocate {

    use cosmwasm_std::{Addr, Decimal, Uint128, Uint256};
    use cw_multi_test::{IntoBech32, StakingSudo};
    use injective_staker::{ONE_INJ, SHARE_PRICE_SCALING_FACTOR};

    use crate::helpers::{
        allocate, assert_error, assert_event_with_attributes, get_allocations, get_share_price,
        get_share_price_num_denom, get_total_allocated, instantiate_staker,
        instantiate_staker_with_min_deposit_and_initial_stake, move_days_forward, pause,
        set_up_allocation,
    };

    #[test]
    fn test_first_allocation() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &anyone,
        );
        let allocate_res =
            allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();
        let (share_price_num, share_price_denom) = get_share_price_num_denom(&app, &contract_addr);
        assert_event_with_attributes(
            &allocate_res.events,
            "wasm-allocated",
            vec![
                ("user", anyone.to_string()).into(),
                ("recipient", recipient.to_string()).into(),
                ("amount", Uint128::from(ONE_INJ)).into(),
                ("total_amount", Uint128::from(ONE_INJ)).into(),
                ("share_price_num", share_price_num).into(),
                ("share_price_denom", share_price_denom).into(),
                ("total_allocated_amount", Uint128::from(ONE_INJ)).into(),
                ("total_allocated_share_price_num", share_price_num).into(),
                ("total_allocated_share_price_denom", share_price_denom).into(),
            ],
            contract_addr,
        );
    }

    #[test]
    fn test_multiple_recipients() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();
        let recipient2: Addr = "recipient2".into_bech32();
        let recipient3: Addr = "recipient3".into_bech32();

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &anyone,
        );

        allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();
        allocate(&mut app, &anyone, &contract_addr, ONE_INJ * 2, &recipient2).unwrap();
        allocate(&mut app, &anyone, &contract_addr, ONE_INJ * 4, &recipient3).unwrap();

        let (share_price_num, share_price_denom) = get_share_price_num_denom(&app, &contract_addr);

        let allocations = get_allocations(&app, &contract_addr, &anyone);
        assert_eq!(allocations.len(), 3);

        let first_allocation = allocations
            .iter()
            .find(|a| a.recipient == recipient)
            .unwrap();
        assert_eq!(first_allocation.recipient, recipient);
        assert_eq!(first_allocation.inj_amount.u128(), ONE_INJ);
        assert_eq!(first_allocation.allocator, anyone);
        assert_eq!(first_allocation.share_price_num, share_price_num);
        assert_eq!(first_allocation.share_price_denom, share_price_denom);

        let second_allocation = allocations
            .iter()
            .find(|a| a.recipient == recipient2)
            .unwrap();
        assert_eq!(second_allocation.recipient, recipient2);
        assert_eq!(second_allocation.inj_amount.u128(), ONE_INJ * 2);
        assert_eq!(second_allocation.allocator, anyone);
        assert_eq!(second_allocation.share_price_num, share_price_num);
        assert_eq!(second_allocation.share_price_denom, share_price_denom);

        let third_allocation = allocations
            .iter()
            .find(|a| a.recipient == recipient3)
            .unwrap();
        assert_eq!(third_allocation.recipient, recipient3);
        assert_eq!(third_allocation.inj_amount.u128(), ONE_INJ * 4);
        assert_eq!(third_allocation.allocator, anyone);
        assert_eq!(third_allocation.share_price_num, share_price_num);
        assert_eq!(third_allocation.share_price_denom, share_price_denom);
    }

    #[test]
    fn test_multiple_allocations() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let allocator: Addr = "anyone".into_bech32();
        let allocator2: Addr = "allocator2".into_bech32();
        let allocator3: Addr = "allocator3".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &allocator,
        );
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &allocator2,
        );
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &allocator3,
        );

        allocate(&mut app, &allocator, &contract_addr, ONE_INJ, &recipient).unwrap();
        allocate(
            &mut app,
            &allocator2,
            &contract_addr,
            ONE_INJ * 3,
            &allocator,
        )
        .unwrap();
        allocate(&mut app, &allocator3, &contract_addr, ONE_INJ, &allocator2).unwrap();

        let (share_price_num, share_price_denom) = get_share_price_num_denom(&app, &contract_addr);

        let allocations = get_allocations(&app, &contract_addr, &allocator);
        assert_eq!(allocations.len(), 1);
        assert_eq!(allocations[0].recipient, recipient);
        assert_eq!(allocations[0].inj_amount.u128(), ONE_INJ);
        assert_eq!(allocations[0].allocator, allocator);
        assert_eq!(allocations[0].share_price_num, share_price_num);
        assert_eq!(allocations[0].share_price_denom, share_price_denom);

        let allocations_2 = get_allocations(&app, &contract_addr, &allocator2);
        assert_eq!(allocations_2.len(), 1);
        assert_eq!(allocations_2.len(), 1);
        assert_eq!(allocations_2[0].recipient, allocator);
        assert_eq!(allocations_2[0].inj_amount.u128(), 3 * ONE_INJ);
        assert_eq!(allocations_2[0].allocator, allocator2);
        assert_eq!(allocations_2[0].share_price_num, share_price_num);
        assert_eq!(allocations_2[0].share_price_denom, share_price_denom);

        let allocations_3 = get_allocations(&app, &contract_addr, &allocator3);
        assert_eq!(allocations_3.len(), 1);
        assert_eq!(allocations_3.len(), 1);
        assert_eq!(allocations_3.len(), 1);
        assert_eq!(allocations_3[0].recipient, allocator2);
        assert_eq!(allocations_3[0].inj_amount.u128(), ONE_INJ);
        assert_eq!(allocations_3[0].allocator, allocator3);
        assert_eq!(allocations_3[0].share_price_num, share_price_num);
        assert_eq!(allocations_3[0].share_price_denom, share_price_denom);
    }

    #[test]
    fn test_allocate_to_same_person_twice_same_share_price() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &anyone,
        );

        allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();
        allocate(&mut app, &anyone, &contract_addr, ONE_INJ * 2, &recipient).unwrap();

        let share_price = get_share_price(&app, &contract_addr);

        let allocations = get_allocations(&app, &contract_addr, &anyone);
        assert_eq!(allocations.len(), 1);

        assert_eq!(allocations[0].recipient, recipient);
        assert_eq!(allocations[0].inj_amount.u128(), ONE_INJ * 3);
        assert_eq!(allocations[0].allocator, anyone);
        assert_eq!(
            allocations[0].share_price_num / allocations[0].share_price_denom,
            Uint256::from(share_price)
        );
    }

    #[test]
    fn test_allocate_to_same_person_twice_with_slashing() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, validator_addr) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &anyone,
        );

        let pre_share_price = get_share_price(&app, &contract_addr);

        allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();

        // Slash the validator by 50%
        app.sudo(cw_multi_test::SudoMsg::Staking(StakingSudo::Slash {
            validator: validator_addr.to_string(),
            percentage: Decimal::percent(50),
        }))
        .unwrap();

        allocate(&mut app, &anyone, &contract_addr, ONE_INJ * 2, &recipient).unwrap();

        let share_price = get_share_price(&app, &contract_addr);

        let allocations = get_allocations(&app, &contract_addr, &anyone);
        assert_eq!(allocations.len(), 1);

        assert_eq!(allocations[0].recipient, recipient);
        assert_eq!(allocations[0].inj_amount.u128(), ONE_INJ * 3);
        assert_eq!(allocations[0].allocator, anyone);
        assert!(
            allocations[0].share_price_num / allocations[0].share_price_denom > share_price.into()
        );
        assert!(
            allocations[0].share_price_num / allocations[0].share_price_denom
                < pre_share_price.into()
        );
    }

    #[test]
    fn test_allocate_to_self_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &anyone,
        );
        let allocate_res = allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &anyone);
        assert_error(allocate_res, "Cannot allocate to self");
    }

    #[test]
    fn test_allocate_below_one_inj_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &anyone,
        );
        let allocate_res = allocate(&mut app, &anyone, &contract_addr, ONE_INJ / 2, &recipient);
        assert_error(allocate_res, "Cannot allocate under 1 INJ");
    }

    #[test]
    fn test_allocate_non_whitelisted_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) = instantiate_staker(owner, "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        let allocate_res = allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient);
        assert_error(allocate_res, "User not whitelisted");
    }

    #[test]
    fn test_allocate_when_contract_paused_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &anyone,
        );

        pause(&mut app, &contract_addr, &owner);

        let allocate_res = allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient);
        assert_error(allocate_res, "Contract is paused");
    }

    #[test]
    fn test_allocate_with_non_existent_recipient_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = Addr::unchecked("recipient");

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &anyone,
        );

        let allocate_res = allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient);
        assert_error(allocate_res, "Generic error: Error decoding bech32");
    }

    #[test]
    fn test_get_total_allocated_with_one_allocation() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &anyone,
        );

        allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();
        let (amount, allocated_share_price_num, allocated_share_price_denom) =
            get_total_allocated(&app, &contract_addr, &anyone);

        let (share_price_num, share_price_denom) = get_share_price_num_denom(&app, &contract_addr);

        assert_eq!(amount.u128(), ONE_INJ);
        assert_eq!(allocated_share_price_num, share_price_num);
        assert_eq!(allocated_share_price_denom, share_price_denom);
    }

    #[test]
    fn test_total_allocated_for_user_with_no_allocations() {
        let owner = "owner".into_bech32();
        let (app, contract_addr, _) = instantiate_staker(owner, "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();

        let (amount, allocated_share_price_num, allocated_share_price_denom) =
            get_total_allocated(&app, &contract_addr, &anyone);

        assert_eq!(amount.u128(), 0);
        assert_eq!(allocated_share_price_num, Uint256::zero());
        assert_eq!(allocated_share_price_denom, Uint256::zero());
    }

    #[test]
    fn test_total_allocated_for_user_with_many_allocation_at_same_price() {
        // instantiate the contract
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            0,
        );
        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();
        let recipient2: Addr = "recipient2".into_bech32();
        let recipient3: Addr = "recipient3".into_bech32();

        // set up for the allocation
        set_up_allocation(&mut app, &owner, &contract_addr, 100000000, &anyone);
        move_days_forward(&mut app, 1);

        allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();
        allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient2).unwrap();
        allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient3).unwrap();

        let (amount, allocated_share_price_num, allocated_share_price_denom) =
            get_total_allocated(&app, &contract_addr, &anyone);

        let (share_price_num, share_price_denom) = get_share_price_num_denom(&app, &contract_addr);

        assert_eq!(amount.u128(), 3 * ONE_INJ);
        assert_eq!(
            allocated_share_price_num / allocated_share_price_denom,
            share_price_num / share_price_denom
        );
    }

    #[test]
    fn test_total_allocated_for_user_with_allocations_at_different_prices() {
        // instantiate the contract
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            0,
        );
        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();
        let recipient2: Addr = "recipient2".into_bech32();
        let recipient3: Addr = "recipient3".into_bech32();

        // set up for the allocation
        set_up_allocation(&mut app, &owner, &contract_addr, 100000000, &anyone);
        move_days_forward(&mut app, 1);

        let share_price_first_alloc = get_share_price(&app, &contract_addr);
        allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();

        move_days_forward(&mut app, 1);
        let share_price_second_alloc = get_share_price(&app, &contract_addr);
        allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient2).unwrap();

        let (amount, allocated_share_price_num, allocated_share_price_denom) =
            get_total_allocated(&app, &contract_addr, &anyone);

        // calculate expected amount and share price
        let expected_num = ONE_INJ + ONE_INJ;
        let expected_denom_summand1 = Uint256::from(ONE_INJ)
            * Uint256::from(SHARE_PRICE_SCALING_FACTOR)
            / Uint256::from(share_price_first_alloc);
        let expected_denom_summand2 = Uint256::from(ONE_INJ)
            * Uint256::from(SHARE_PRICE_SCALING_FACTOR)
            / Uint256::from(share_price_second_alloc);
        let expected_denom = expected_denom_summand1 + expected_denom_summand2;
        let expected_share_price = Uint256::from(expected_num)
            * Uint256::from(SHARE_PRICE_SCALING_FACTOR)
            / expected_denom;

        assert_eq!(amount.u128(), 2 * ONE_INJ);
        assert_eq!(
            allocated_share_price_num / allocated_share_price_denom,
            expected_share_price
        );

        move_days_forward(&mut app, 1);

        let share_price_third_alloc = get_share_price(&app, &contract_addr);
        allocate(&mut app, &anyone, &contract_addr, ONE_INJ * 5, &recipient3).unwrap();

        let (new_amount, new_allocated_share_price_num, new_allocated_share_price_denom) =
            get_total_allocated(&app, &contract_addr, &anyone);

        // calculate new expected amount and share price
        let expected_new_num = expected_num + ONE_INJ * 5;
        let expected_new_denom_summand1 = Uint256::from(expected_num)
            * Uint256::from(SHARE_PRICE_SCALING_FACTOR)
            / expected_share_price;
        let expected_new_denom_summand2 = Uint256::from(5 * ONE_INJ)
            * Uint256::from(SHARE_PRICE_SCALING_FACTOR)
            / Uint256::from(share_price_third_alloc);
        let expected_new_denom = expected_new_denom_summand1 + expected_new_denom_summand2;
        let expected_new_share_price = Uint256::from(expected_new_num)
            * Uint256::from(SHARE_PRICE_SCALING_FACTOR)
            / expected_new_denom;

        assert_eq!(new_amount.u128(), 7 * ONE_INJ);
        assert_eq!(
            new_allocated_share_price_num / new_allocated_share_price_denom,
            expected_new_share_price
        );
    }
}
