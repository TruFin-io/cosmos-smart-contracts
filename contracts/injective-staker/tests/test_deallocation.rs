pub mod helpers;

#[cfg(test)]
mod deallocate {

    use cosmwasm_std::{Addr, Uint128};
    use cw_multi_test::IntoBech32;
    use injective_staker::ONE_INJ;

    use crate::helpers::{
        allocate, assert_error, assert_event_with_attributes, clear_whitelist_status, deallocate,
        get_allocations, get_share_price_num_denom, instantiate_staker,
        instantiate_staker_with_min_deposit_and_initial_stake, move_days_forward, pause,
        set_up_allocation, whitelist_user,
    };

    #[test]
    fn test_deallocation_reduces_allocation() {
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
        allocate(&mut app, &anyone, &contract_addr, ONE_INJ * 2, &recipient).unwrap();
        let deallocate_res =
            deallocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();

        let allocations = get_allocations(&app, &contract_addr, &anyone);
        assert!(allocations.len() == 1);

        let (share_price_num, share_price_denom) = get_share_price_num_denom(&app, &contract_addr);

        assert_eq!(allocations[0].recipient, recipient);
        assert_eq!(allocations[0].inj_amount.u128(), ONE_INJ);
        assert_eq!(allocations[0].allocator, anyone);
        assert_eq!(allocations[0].share_price_num, share_price_num);
        assert_eq!(allocations[0].share_price_denom, share_price_denom);

        assert_event_with_attributes(
            &deallocate_res.events,
            "wasm-deallocated",
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
    fn test_deallocation_does_not_change_allocation_share_price() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) = instantiate_staker_with_min_deposit_and_initial_stake(
            owner.clone(),
            "treasury".into_bech32(),
            0,
            0,
        );
        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        // set up for the allocation
        set_up_allocation(&mut app, &owner, &contract_addr, 1000000000, &anyone);

        allocate(&mut app, &anyone, &contract_addr, ONE_INJ * 2, &recipient).unwrap();
        let (pre_share_price_num, pre_share_price_denom) =
            get_share_price_num_denom(&app, &contract_addr);

        move_days_forward(&mut app, 1);
        deallocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();

        let (post_share_price_num, post_share_price_denom) =
            get_share_price_num_denom(&app, &contract_addr);

        assert!(
            post_share_price_num / post_share_price_denom
                > pre_share_price_num / pre_share_price_denom
        );

        let allocations = get_allocations(&app, &contract_addr, &anyone);
        assert!(allocations.len() == 1);

        assert_eq!(allocations[0].recipient, recipient);
        assert_eq!(allocations[0].inj_amount.u128(), ONE_INJ);
        assert_eq!(allocations[0].allocator, anyone);
        assert_eq!(allocations[0].share_price_num, pre_share_price_num);
        assert_eq!(allocations[0].share_price_denom, pre_share_price_denom);
    }

    #[test]
    fn test_deallocating_full_amount_removes_allocation() {
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
        deallocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();

        let allocations = get_allocations(&app, &contract_addr, &anyone);
        assert_eq!(allocations.len(), 0);
    }

    #[test]
    fn test_deallocating_with_no_allocations_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        let deallocation_res = deallocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient);
        assert_error(deallocation_res, "No Allocation to recipient");
    }

    #[test]
    fn test_deallocating_non_existent_allocation_fails() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();
        let noone: Addr = "noone".into_bech32();

        // set up for the allocation
        set_up_allocation(
            &mut app,
            &owner,
            &contract_addr,
            10000000000000000000,
            &anyone,
        );

        allocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient).unwrap();
        let deallocation_res = deallocate(&mut app, &anyone, &contract_addr, ONE_INJ, &noone);

        assert_error(deallocation_res, "No Allocation to recipient");
    }

    #[test]
    fn test_deallocating_excessive_amount_fails() {
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
        let deallocation_res =
            deallocate(&mut app, &anyone, &contract_addr, ONE_INJ * 2, &recipient);

        assert_error(deallocation_res, "Cannot deallocate more than is allocated");
    }

    #[test]
    fn test_deallocating_to_less_than_one_near_fails() {
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
        let deallocation_res =
            deallocate(&mut app, &anyone, &contract_addr, ONE_INJ / 2, &recipient);

        assert_error(deallocation_res, "Cannot allocate under 1 INJ");
    }

    #[test]
    fn test_deallocate_not_whitelisted_fails() {
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

        clear_whitelist_status(&mut app, &contract_addr, &owner, &anyone);
        let deallocation_res = deallocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient);

        assert_error(deallocation_res, "User not whitelisted");
    }

    #[test]
    fn test_deallocate_when_contract_paused_fails() {
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

        pause(&mut app, &contract_addr, &owner);
        let deallocation_res = deallocate(&mut app, &anyone, &contract_addr, ONE_INJ, &recipient);

        assert_error(deallocation_res, "Contract is paused");
    }
}
