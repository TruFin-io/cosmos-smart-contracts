pub mod helpers;

#[cfg(test)]
mod compound_rewards {

    use cosmwasm_std::{to_json_binary, Addr, Uint128, WasmMsg};
    use cw_multi_test::{Executor, IntoBech32};
    use helpers::{mint_inj, stake};
    use injective_staker::{msg::ExecuteMsg, FEE_PRECISION, SHARE_PRICE_SCALING_FACTOR};

    use crate::helpers::{
        self, add_validator, assert_event_with_attributes, get_share_price, get_total_rewards,
        get_total_staked, instantiate_staker, move_days_forward, query_truinj_balance,
        set_min_deposit_for_test_overflow, stake_to_specific_validator, whitelist_user,
    };

    #[test]
    fn test_compound_rewards() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 1000000000;
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        set_min_deposit_for_test_overflow(&mut app, contract_addr.to_string(), owner.clone(), 0);
        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        move_days_forward(&mut app, 1);

        // query total staked and rewards after reward distribution
        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        let msg = ExecuteMsg::CompoundRewards;

        let cosmos_msg = WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&msg).unwrap(),
            funds: vec![],
        };
        let restake_res = app.execute(anyone.clone(), cosmos_msg.into()).unwrap();
        let new_total_staked = get_total_staked(&app, &contract_addr);
        let new_total_rewards = get_total_rewards(&app, &contract_addr);

        assert!(total_rewards.u128() > 0);
        assert!(new_total_rewards.is_zero());
        assert!(new_total_staked == total_staked + total_rewards);

        assert_event_with_attributes(
            &restake_res.events,
            "wasm-restaked",
            vec![
                ("amount", total_rewards).into(),
                ("treasury_shares_minted", Uint128::zero()).into(),
                ("treasury_balance", Uint128::zero()).into(),
            ],
            contract_addr,
        );
    }

    #[test]
    fn test_compound_rewards_multi_validators() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 1000000000000;
        mint_inj(&mut app, &anyone, inj_to_mint * 3);

        set_min_deposit_for_test_overflow(&mut app, contract_addr.to_string(), owner.clone(), 0);

        let second_validator: Addr = "second_validator".into_bech32();
        add_validator(
            &mut app,
            owner.clone(),
            &contract_addr,
            second_validator.clone(),
        )
        .unwrap();

        let third_validator: Addr = "third_validator".into_bech32();
        add_validator(
            &mut app,
            owner.clone(),
            &contract_addr,
            third_validator.clone(),
        )
        .unwrap();

        let fourth_validator: Addr = "fourth_validator".into_bech32();
        add_validator(&mut app, owner.clone(), &contract_addr, fourth_validator).unwrap();

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stakes
        stake_to_specific_validator(
            &mut app,
            &anyone,
            &contract_addr,
            inj_to_mint,
            &second_validator,
        )
        .unwrap();

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        move_days_forward(&mut app, 1);

        stake_to_specific_validator(
            &mut app,
            &anyone,
            &contract_addr,
            inj_to_mint,
            &third_validator,
        )
        .unwrap();

        // query total staked and rewards after reward distribution
        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        let msg = ExecuteMsg::CompoundRewards;

        let cosmos_msg = WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&msg).unwrap(),
            funds: vec![],
        };
        let restake_res = app.execute(anyone.clone(), cosmos_msg.into()).unwrap();
        let new_total_staked = get_total_staked(&app, &contract_addr);
        let new_total_rewards = get_total_rewards(&app, &contract_addr);

        assert!(total_rewards.u128() > 0);
        assert!(new_total_rewards.is_zero());
        assert!(new_total_staked == total_staked + total_rewards);

        assert_event_with_attributes(
            &restake_res.events,
            "wasm-restaked",
            vec![
                ("amount", total_rewards).into(),
                ("treasury_shares_minted", Uint128::zero()).into(),
                ("treasury_balance", Uint128::zero()).into(),
            ],
            contract_addr,
        );
    }

    #[test]
    fn test_compound_rewards_with_fees() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, contract_addr, _) = instantiate_staker(owner.clone(), treasury.clone());

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 100000000000;
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        set_min_deposit_for_test_overflow(&mut app, contract_addr.to_string(), owner.clone(), 0);
        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        move_days_forward(&mut app, 1);

        // set fee for treasury
        let msg = WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::SetFee { new_fee: 1000 }).unwrap(),
            funds: vec![],
        };
        app.execute(owner, msg.into()).unwrap();

        // query total staked and rewards after reward distribution
        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        let treasury_pre_balance = query_truinj_balance(&app, &treasury, &contract_addr);

        let msg = ExecuteMsg::CompoundRewards;

        let cosmos_msg = WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&msg).unwrap(),
            funds: vec![],
        };
        let restake_res = app.execute(anyone.clone(), cosmos_msg.into()).unwrap();
        let new_total_staked = get_total_staked(&app, &contract_addr);
        let new_total_rewards = get_total_rewards(&app, &contract_addr);

        let treasury_post_balance = query_truinj_balance(&app, &treasury, &contract_addr);
        let share_price = get_share_price(&app, &contract_addr);
        let treasury_fees = total_rewards.u128() * 1000 / u128::from(FEE_PRECISION);
        let treasury_fee_shares = (treasury_fees * SHARE_PRICE_SCALING_FACTOR) / share_price;

        assert!(total_rewards.u128() > 0);
        assert!(new_total_rewards.is_zero());
        assert!(new_total_staked == total_staked + total_rewards);
        assert!(treasury_post_balance > treasury_pre_balance);
        assert!(treasury_post_balance == treasury_pre_balance + treasury_fee_shares);

        assert_event_with_attributes(
            &restake_res.events,
            "wasm-restaked",
            vec![
                ("amount", total_rewards).into(),
                ("treasury_shares_minted", Uint128::from(treasury_fee_shares)).into(),
                ("treasury_balance", Uint128::from(treasury_fee_shares)).into(),
            ],
            contract_addr.clone(),
        );

        assert_event_with_attributes(
            &restake_res.events,
            "wasm",
            vec![
                ("action", "mint").into(),
                ("to", treasury.to_string()).into(),
                ("amount", Uint128::from(treasury_fee_shares)).into(),
            ],
            contract_addr,
        );
    }

    #[test]
    fn test_compound_rewards_when_no_rewards_have_accrued() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, contract_addr, _) = instantiate_staker(owner.clone(), treasury.clone());

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 1000000000000000000;
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        // set fee for treasury
        let msg = WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::SetFee { new_fee: 1000 }).unwrap(),
            funds: vec![],
        };
        app.execute(owner, msg.into()).unwrap();

        // query total staked and rewards after reward distribution
        let total_staked = get_total_staked(&app, &contract_addr);
        let total_rewards = get_total_rewards(&app, &contract_addr);

        let treasury_pre_balance = query_truinj_balance(&app, &treasury, &contract_addr);

        let msg = ExecuteMsg::CompoundRewards;

        let cosmos_msg = WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&msg).unwrap(),
            funds: vec![],
        };
        app.execute(anyone.clone(), cosmos_msg.into()).unwrap();

        let new_total_staked = get_total_staked(&app, &contract_addr);
        let new_total_rewards = get_total_rewards(&app, &contract_addr);
        let treasury_post_balance = query_truinj_balance(&app, &treasury, &contract_addr);

        assert!(total_rewards.is_zero());
        assert!(new_total_rewards.is_zero());
        assert!(new_total_staked == total_staked);
        assert!(treasury_post_balance == treasury_pre_balance);
    }
}
