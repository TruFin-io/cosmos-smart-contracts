pub mod helpers;

#[cfg(test)]
mod distribute_all {

    use cosmwasm_std::{coin, to_json_binary, Addr, Event, WasmMsg};
    use cw_multi_test::{App, Executor, IntoBech32};
    use injective_staker::{msg::ExecuteMsg, INJ, ONE_INJ};

    use crate::helpers::{
        assert_error, assert_event, assert_event_with_attributes, clear_whitelist_status,
        distribute_rewards, find_event_for_recipient, get_distribution_amounts,
        get_share_price_num_denom, get_total_allocated, instantiate_staker_with_min_deposit,
        mint_inj, move_days_forward, pause, query_inj_balance, query_truinj_balance, set_dist_fee,
        set_up_test_allocation, transfer_truinj, whitelist_user,
    };

    #[test]
    fn test_distribute_all_in_truinj() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), treasury.clone(), 0);

        set_dist_fee(&mut app, &staker_addr, &owner, 500); // 5%

        let (distributor, first_recipient, second_recipient) =
            setup_allocations(&mut app, &staker_addr, &owner);

        // accrue rewards
        move_days_forward(&mut app, 30);

        // check balances before distribution
        let pre_distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);

        let (dist_share_price_num, dist_share_price_denom) =
            get_share_price_num_denom(&app, &staker_addr);

        let (first_inj_to_distribute, first_truinj_to_distribute, first_fees) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&first_recipient));

        let (second_inj_to_distribute, second_truinj_to_distribute, second_fees) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&second_recipient));

        // distribute all in truinj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: false }).unwrap(),
                funds: vec![],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // get the balances after distribution
        let distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        let first_recipient_truinj_balance =
            query_truinj_balance(&app, &first_recipient, &staker_addr);
        let second_recipient_truinj_balance =
            query_truinj_balance(&app, &second_recipient, &staker_addr);
        let treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);

        // verify the distributor truinj balance was reduced by the expected amount
        let total_truinj_spent =
            first_truinj_to_distribute + second_truinj_to_distribute + first_fees + second_fees;
        assert_eq!(
            distributor_truinj_balance,
            pre_distributor_truinj_balance - total_truinj_spent
        );

        // verify the recipients received the expected amount of shares
        assert_eq!(first_recipient_truinj_balance, first_truinj_to_distribute);
        assert_eq!(second_recipient_truinj_balance, second_truinj_to_distribute);

        // verify the treasury received the expected amount of fees
        assert_eq!(treasury_truinj_balance, first_fees + second_fees);

        let (
            total_allocated_amount,
            total_allocated_share_price_num,
            total_allocated_share_price_denom,
        ) = get_total_allocated(&app, &staker_addr, &distributor);

        // verify both distributed_rewards events were emitted
        let events: Vec<Event> = dist_res.unwrap().events;
        let balance_after_first_distribution =
            distributor_truinj_balance + second_inj_to_distribute + second_fees - 3;
        let first_recipient_event = |event: &Event| {
            event
                .attributes
                .iter()
                .any(|attr| attr == ("recipient", first_recipient.to_string()))
        };
        assert_event(
            &events,
            "wasm-distributed_rewards",
            first_recipient_event,
            vec![
                ("user", distributor.to_string()).into(),
                ("recipient", first_recipient.to_string()).into(),
                ("user_balance", balance_after_first_distribution.to_string()).into(),
                (
                    "recipient_balance",
                    first_recipient_truinj_balance.to_string(),
                )
                    .into(),
                ("treasury_balance", first_fees.to_string()).into(),
                ("fees", first_fees.to_string()).into(),
                ("shares", first_truinj_to_distribute.to_string()).into(),
                ("inj_amount", first_inj_to_distribute.to_string()).into(),
                ("in_inj", "false").into(),
                ("share_price_num", dist_share_price_num).into(),
                ("share_price_denom", dist_share_price_denom).into(),
                ("total_allocated_amount", total_allocated_amount).into(),
                (
                    "total_allocated_share_price_num",
                    total_allocated_share_price_num,
                )
                    .into(),
                (
                    "total_allocated_share_price_denom",
                    total_allocated_share_price_denom,
                )
                    .into(),
            ],
            staker_addr.clone(),
        );

        let second_recipient_event = |event: &Event| {
            event
                .attributes
                .iter()
                .any(|attr| attr == ("recipient", second_recipient.to_string()))
        };
        assert_event(
            &events,
            "wasm-distributed_rewards",
            second_recipient_event,
            vec![
                ("user", distributor.to_string()).into(),
                ("recipient", second_recipient.to_string()).into(),
                ("user_balance", distributor_truinj_balance.to_string()).into(),
                (
                    "recipient_balance",
                    second_recipient_truinj_balance.to_string(),
                )
                    .into(),
                ("treasury_balance", treasury_truinj_balance.to_string()).into(),
                ("fees", second_fees.to_string()).into(),
                ("shares", second_truinj_to_distribute.to_string()).into(),
                ("inj_amount", second_inj_to_distribute.to_string()).into(),
                ("in_inj", "false").into(),
                ("share_price_num", dist_share_price_num).into(),
                ("share_price_denom", dist_share_price_denom).into(),
                ("total_allocated_amount", total_allocated_amount).into(),
                (
                    "total_allocated_share_price_num",
                    total_allocated_share_price_num,
                )
                    .into(),
                (
                    "total_allocated_share_price_denom",
                    total_allocated_share_price_denom,
                )
                    .into(),
            ],
            staker_addr.clone(),
        );

        // verify distributed all event was emitted
        assert_event_with_attributes(
            &events,
            "wasm-distributed_all",
            vec![("user", distributor.to_string()).into()],
            staker_addr,
        );
    }

    #[test]
    fn test_distribute_all_in_truinj_with_no_rewards_to_distribute() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), treasury.clone(), 0);

        set_dist_fee(&mut app, &staker_addr, &owner, 500); // 5%

        let (distributor, first_recipient, second_recipient) =
            setup_allocations(&mut app, &staker_addr, &owner);

        let pre_distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);

        // distribute all in truinj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: false }).unwrap(),
                funds: vec![],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // get the balances after distribution
        let distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        let first_recipient_truinj_balance =
            query_truinj_balance(&app, &first_recipient, &staker_addr);
        let second_recipient_truinj_balance =
            query_truinj_balance(&app, &second_recipient, &staker_addr);
        let treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);

        // verify the distributor truinj balance is unchanged
        assert_eq!(distributor_truinj_balance, pre_distributor_truinj_balance);

        // verify the recipients balances are unchanged
        assert_eq!(first_recipient_truinj_balance, 0);
        assert_eq!(second_recipient_truinj_balance, 0);

        // verify the treasury received no fees
        assert_eq!(treasury_truinj_balance, 0);

        // verify no distributed_rewards events were emitted
        let events: Vec<Event> = dist_res.unwrap().events;
        assert!(!events
            .iter()
            .any(|p: &_| p.ty == "wasm-distributed_rewards"));

        // verify distributed all event was emitted
        assert_event_with_attributes(
            &events,
            "wasm-distributed_all",
            vec![("user", distributor.to_string()).into()],
            staker_addr,
        );
    }

    #[test]
    fn test_distribute_all_in_truinj_refunds_all_inj_attached() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        set_dist_fee(&mut app, &staker_addr, &owner, 500); // 5%

        let (distributor, _, _) = setup_allocations(&mut app, &staker_addr, &owner);

        // accrue rewards
        move_days_forward(&mut app, 30);

        // mint to the distributor some inj
        let inj_to_attach = 1_000;
        mint_inj(&mut app, &distributor, inj_to_attach);

        // get the distributor inj balance before distribution
        let pre_distributor_inj_balance = query_inj_balance(&app, &distributor);

        // distribute all in truinj attaching some inj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: false }).unwrap(),
                funds: vec![coin(inj_to_attach, INJ)],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // verify the distributor was refunded all the inj attached
        let distributor_inj_balance = query_inj_balance(&app, &distributor);
        assert_eq!(distributor_inj_balance, pre_distributor_inj_balance);
    }

    #[test]
    fn test_distribute_all_in_truinj_to_many_recipients_some_already_distributed_to() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let recipients = vec![
            "recipient0".into_bech32(),
            "recipient1".into_bech32(),
            "recipient2".into_bech32(),
            "recipient3".into_bech32(),
            "recipient4".into_bech32(),
        ];
        let distributor =
            setup_allocations_to_recipients(&mut app, &owner, &recipients, &staker_addr);

        // accrue rewards
        move_days_forward(&mut app, 30);

        // distribute rewards to recipients[0] and recipients[1] in truinj
        distribute_rewards(
            &mut app,
            &staker_addr,
            &distributor,
            &recipients[0],
            false,
            None,
        );
        distribute_rewards(
            &mut app,
            &staker_addr,
            &distributor,
            &recipients[1],
            false,
            None,
        );

        // distribute rewards to recipient[4] in inj
        mint_inj(&mut app, &distributor, ONE_INJ);
        distribute_rewards(
            &mut app,
            &staker_addr,
            &distributor,
            &recipients[4],
            true,
            Some(ONE_INJ),
        );

        // get pre distribution balances
        let pre_recipient0_balance = query_truinj_balance(&app, &recipients[0], &staker_addr);
        let pre_recipient1_balance = query_truinj_balance(&app, &recipients[1], &staker_addr);
        let pre_recipient2_balance = query_truinj_balance(&app, &recipients[2], &staker_addr);
        let pre_recipient3_balance = query_truinj_balance(&app, &recipients[3], &staker_addr);
        let pre_recipient4_balance = query_truinj_balance(&app, &recipients[4], &staker_addr);

        // get the truinj amounts to distribute for recipients[2] and recipients[3]
        let (_, truinj_to_distribute, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, None);
        let (_, recipient2_dist_amount, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipients[2]));
        let (_, recipient3_dist_amount, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipients[3]));

        // verify the total amount to distribute is the sum of the amounts to distribute to the recipients
        assert_eq!(
            truinj_to_distribute,
            recipient2_dist_amount + recipient3_dist_amount
        );

        // distribute all in truinj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: false }).unwrap(),
                funds: vec![],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // get the balances after distribution
        let recipient0_balance = query_truinj_balance(&app, &recipients[0], &staker_addr);
        let recipient1_balance = query_truinj_balance(&app, &recipients[1], &staker_addr);
        let recipient2_balance = query_truinj_balance(&app, &recipients[2], &staker_addr);
        let recipient3_balance = query_truinj_balance(&app, &recipients[3], &staker_addr);
        let recipient4_balance = query_truinj_balance(&app, &recipients[4], &staker_addr);

        // verify that the recipients that were already distributed to received no additional rewards
        assert_eq!(recipient0_balance, pre_recipient0_balance);
        assert_eq!(recipient1_balance, pre_recipient1_balance);
        assert_eq!(recipient4_balance, pre_recipient4_balance);

        // verify that recipient[2] and recipient[3] received the expected rewards
        assert_eq!(
            recipient2_balance,
            pre_recipient2_balance + recipient2_dist_amount
        );
        assert_eq!(
            recipient3_balance,
            pre_recipient3_balance + recipient3_dist_amount
        );

        // verify no distributed_rewards events were emitted for the recipients that the user already distributed to
        let all_events = dist_res.unwrap().events;
        let dist_reward_events = all_events
            .iter()
            .filter(|p: &&_| p.ty == "wasm-distributed_rewards")
            .cloned()
            .collect::<Vec<Event>>();

        assert!(find_event_for_recipient(&dist_reward_events, &recipients[0]).is_none());
        assert!(find_event_for_recipient(&dist_reward_events, &recipients[1]).is_none());
        assert!(find_event_for_recipient(&dist_reward_events, &recipients[2]).is_some());
        assert!(find_event_for_recipient(&dist_reward_events, &recipients[3]).is_some());
        assert!(find_event_for_recipient(&dist_reward_events, &recipients[4]).is_none());
    }

    #[test]
    fn test_distribute_all_in_inj() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), treasury.clone(), 0);

        set_dist_fee(&mut app, &staker_addr, &owner, 500); // 5%

        let (distributor, first_recipient, second_recipient) =
            setup_allocations(&mut app, &staker_addr, &owner);

        // accrue rewards
        move_days_forward(&mut app, 30);

        let (dist_share_price_num, dist_share_price_denom) =
            get_share_price_num_denom(&app, &staker_addr);

        let (first_inj_to_distribute, first_truinj_to_distribute, first_fees) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&first_recipient));

        let (second_inj_to_distribute, second_truinj_to_distribute, second_fees) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&second_recipient));

        // mint to the distributor the exact amount of inj that will be distributed
        let inj_to_distribute = first_inj_to_distribute + second_inj_to_distribute;
        mint_inj(&mut app, &distributor, inj_to_distribute);
        let pre_distributor_inj_balance = query_inj_balance(&app, &distributor);

        // distribute all in inj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: true }).unwrap(),
                funds: vec![coin(inj_to_distribute, INJ)],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // get the balances after distribution
        let distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        let distributor_inj_balance = query_inj_balance(&app, &distributor);
        let first_recipient_inj_balance = query_inj_balance(&app, &first_recipient);
        let first_recipient_truinj_balance =
            query_truinj_balance(&app, &first_recipient, &staker_addr);
        let second_recipient_inj_balance = query_inj_balance(&app, &second_recipient);
        let second_recipient_truinj_balance =
            query_truinj_balance(&app, &second_recipient, &staker_addr);
        let treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);

        // verify the distributor inj balance was reduced by the expected amount
        assert_eq!(
            distributor_inj_balance,
            pre_distributor_inj_balance - inj_to_distribute
        );

        // verify the recipients received the expected amount of inj
        assert_eq!(first_recipient_inj_balance, first_inj_to_distribute);
        assert_eq!(second_recipient_inj_balance, second_inj_to_distribute);

        // verify the treasury received the expected amount of fees
        assert_eq!(treasury_truinj_balance, first_fees + second_fees);

        let (
            total_allocated_amount,
            total_allocated_share_price_num,
            total_allocated_share_price_denom,
        ) = get_total_allocated(&app, &staker_addr, &distributor);

        // verify both distributed_rewards events were emitted
        let events: Vec<Event> = dist_res.unwrap().events;
        let distributor_truinj_after_first_distribution = distributor_truinj_balance + second_fees;

        let first_distribution_event = |event: &Event| {
            event
                .attributes
                .iter()
                .any(|attr| attr == ("recipient", first_recipient.to_string()))
        };
        assert_event(
            &events,
            "wasm-distributed_rewards",
            first_distribution_event,
            vec![
                ("user", distributor.to_string()).into(),
                ("recipient", first_recipient.to_string()).into(),
                (
                    "user_balance",
                    distributor_truinj_after_first_distribution.to_string(),
                )
                    .into(),
                (
                    "recipient_balance",
                    first_recipient_truinj_balance.to_string(),
                )
                    .into(),
                ("treasury_balance", first_fees.to_string()).into(),
                ("fees", first_fees.to_string()).into(),
                ("shares", first_truinj_to_distribute.to_string()).into(),
                ("inj_amount", first_inj_to_distribute.to_string()).into(),
                ("in_inj", "true").into(),
                ("share_price_num", dist_share_price_num).into(),
                ("share_price_denom", dist_share_price_denom).into(),
                ("total_allocated_amount", total_allocated_amount).into(),
                (
                    "total_allocated_share_price_num",
                    total_allocated_share_price_num,
                )
                    .into(),
                (
                    "total_allocated_share_price_denom",
                    total_allocated_share_price_denom,
                )
                    .into(),
            ],
            staker_addr.clone(),
        );

        let second_distribution_event = |event: &Event| {
            event
                .attributes
                .iter()
                .any(|attr| attr == ("recipient", second_recipient.to_string()))
        };
        assert_event(
            &events,
            "wasm-distributed_rewards",
            second_distribution_event,
            vec![
                ("user", distributor.to_string()).into(),
                ("recipient", second_recipient.to_string()).into(),
                ("user_balance", distributor_truinj_balance.to_string()).into(),
                (
                    "recipient_balance",
                    second_recipient_truinj_balance.to_string(),
                )
                    .into(),
                ("treasury_balance", treasury_truinj_balance.to_string()).into(),
                ("fees", second_fees.to_string()).into(),
                ("shares", second_truinj_to_distribute.to_string()).into(),
                ("inj_amount", second_inj_to_distribute.to_string()).into(),
                ("in_inj", "true").into(),
                ("share_price_num", dist_share_price_num).into(),
                ("share_price_denom", dist_share_price_denom).into(),
                ("total_allocated_amount", total_allocated_amount).into(),
                (
                    "total_allocated_share_price_num",
                    total_allocated_share_price_num,
                )
                    .into(),
                (
                    "total_allocated_share_price_denom",
                    total_allocated_share_price_denom,
                )
                    .into(),
            ],
            staker_addr.clone(),
        );

        // verify distributed all event was emitted
        assert_event_with_attributes(
            &events,
            "wasm-distributed_all",
            vec![("user", distributor.to_string()).into()],
            staker_addr,
        );
    }

    #[test]
    fn test_distribute_all_in_inj_refunds_excess_inj_attached() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        set_dist_fee(&mut app, &staker_addr, &owner, 500); // 5%

        let (distributor, first_recipient, second_recipient) =
            setup_allocations(&mut app, &staker_addr, &owner);

        // accrue rewards
        move_days_forward(&mut app, 30);

        let (first_inj_to_distribute, _, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&first_recipient));

        let (second_inj_to_distribute, _, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&second_recipient));

        // mint to the distributor more inj than it will be distributed
        let excess_inj = 1000;
        let inj_to_attach = first_inj_to_distribute + second_inj_to_distribute + excess_inj;
        mint_inj(&mut app, &distributor, inj_to_attach);

        // distribute all in inj with excess inj attached
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: true }).unwrap(),
                funds: vec![coin(inj_to_attach, INJ)],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // get the balances after distribution
        let distributor_inj_balance = query_inj_balance(&app, &distributor);
        let first_recipient_inj_balance = query_inj_balance(&app, &first_recipient);
        let second_recipient_inj_balance = query_inj_balance(&app, &second_recipient);

        // verify the distributor was refunded the excess inj attached
        assert_eq!(distributor_inj_balance, excess_inj);

        // verify the recipients received the expected amount of inj
        assert_eq!(first_recipient_inj_balance, first_inj_to_distribute);
        assert_eq!(second_recipient_inj_balance, second_inj_to_distribute);
    }

    #[test]
    fn test_distribute_all_in_inj_with_no_rewards_to_distribute() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), treasury.clone(), 0);

        set_dist_fee(&mut app, &staker_addr, &owner, 500); // 5%

        let (distributor, first_recipient, second_recipient) =
            setup_allocations(&mut app, &staker_addr, &owner);

        let pre_distributor_inj_balance = query_inj_balance(&app, &distributor);

        // distribute all in truinj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: true }).unwrap(),
                funds: vec![],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // get the balances after distribution
        let distributor_inj_balance = query_inj_balance(&app, &distributor);
        let first_recipient_inj_balance = query_inj_balance(&app, &first_recipient);
        let second_recipient_inj_balance = query_inj_balance(&app, &second_recipient);
        let treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);

        // verify the distributor inj balance is unchanged
        assert_eq!(distributor_inj_balance, pre_distributor_inj_balance);

        // verify the recipients inj balances are unchanged
        assert_eq!(first_recipient_inj_balance, 0);
        assert_eq!(second_recipient_inj_balance, 0);

        // verify the treasury received no fees
        assert_eq!(treasury_truinj_balance, 0);

        // verify no distributed_rewards events were emitted
        let events: Vec<Event> = dist_res.unwrap().events;
        assert!(!events
            .iter()
            .any(|p: &_| p.ty == "wasm-distributed_rewards"));

        // verify distributed all event was emitted
        assert_event_with_attributes(
            &events,
            "wasm-distributed_all",
            vec![("user", distributor.to_string()).into()],
            staker_addr,
        );
    }

    #[test]
    fn test_distribute_all_in_inj_to_many_recipients_some_already_distributed_to() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let recipients = vec![
            "recipient0".into_bech32(),
            "recipient1".into_bech32(),
            "recipient2".into_bech32(),
            "recipient3".into_bech32(),
            "recipient4".into_bech32(),
        ];
        let distributor =
            setup_allocations_to_recipients(&mut app, &owner, &recipients, &staker_addr);

        // accrue rewards
        move_days_forward(&mut app, 30);

        // distribute rewards to recipients[0] and recipients[1] in truinj
        distribute_rewards(
            &mut app,
            &staker_addr,
            &distributor,
            &recipients[0],
            false,
            None,
        );
        distribute_rewards(
            &mut app,
            &staker_addr,
            &distributor,
            &recipients[1],
            false,
            None,
        );

        // distribute rewards to recipients[4] in inj
        let (recipient4_dist_amount, _, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipients[4]));
        mint_inj(&mut app, &distributor, recipient4_dist_amount);
        distribute_rewards(
            &mut app,
            &staker_addr,
            &distributor,
            &recipients[4],
            true,
            Some(recipient4_dist_amount),
        );

        // get pre distribution inj balances
        let pre_recipient0_balance = query_inj_balance(&app, &recipients[0]);
        let pre_recipient1_balance = query_inj_balance(&app, &recipients[1]);
        let pre_recipient2_balance = query_inj_balance(&app, &recipients[2]);
        let pre_recipient3_balance = query_inj_balance(&app, &recipients[3]);
        let pre_recipient4_balance = query_inj_balance(&app, &recipients[4]);

        // get the inj amount to distribute all
        let (inj_to_distribute, _, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, None);
        mint_inj(&mut app, &distributor, inj_to_distribute);

        let (recipient2_dist_amount, _, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipients[2]));
        let (recipient3_dist_amount, _, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipients[3]));

        assert_eq!(
            inj_to_distribute,
            recipient2_dist_amount + recipient3_dist_amount
        );
        // distribute all in inj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: true }).unwrap(),
                funds: vec![coin(inj_to_distribute, INJ)],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // verify the distributor spent all the inj attached
        let distributor_inj_balance = query_inj_balance(&app, &distributor);
        assert_eq!(distributor_inj_balance, 0);

        // get the inj balances after distribution
        let recipient0_balance = query_inj_balance(&app, &recipients[0]);
        let recipient1_balance = query_inj_balance(&app, &recipients[1]);
        let recipient2_balance = query_inj_balance(&app, &recipients[2]);
        let recipient3_balance = query_inj_balance(&app, &recipients[3]);
        let recipient4_balance = query_inj_balance(&app, &recipients[4]);

        // verify that the recipients that were already distributed to received no additional rewards
        assert_eq!(recipient0_balance, pre_recipient0_balance);
        assert_eq!(recipient1_balance, pre_recipient1_balance);
        assert_eq!(recipient4_balance, pre_recipient4_balance);

        // verify that recipient[2] and recipient[3] received the expected rewards
        assert_eq!(
            recipient2_balance,
            pre_recipient2_balance + recipient2_dist_amount
        );
        assert_eq!(
            recipient3_balance,
            pre_recipient3_balance + recipient3_dist_amount
        );

        // verify no distributed_rewards events were emitted for the recipients that the user already distributed to
        let all_events = dist_res.unwrap().events;
        let dist_reward_events = all_events
            .iter()
            .filter(|p: &&_| p.ty == "wasm-distributed_rewards")
            .cloned()
            .collect::<Vec<Event>>();

        assert!(find_event_for_recipient(&dist_reward_events, &recipients[0]).is_none());
        assert!(find_event_for_recipient(&dist_reward_events, &recipients[1]).is_none());
        assert!(find_event_for_recipient(&dist_reward_events, &recipients[2]).is_some());
        assert!(find_event_for_recipient(&dist_reward_events, &recipients[3]).is_some());
        assert!(find_event_for_recipient(&dist_reward_events, &recipients[4]).is_none());
    }

    #[test]
    fn test_distribute_all_when_user_not_whitelisted_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let (distributor, _, _) = setup_allocations(&mut app, &staker_addr, &owner);

        // remove user from whitelist
        clear_whitelist_status(&mut app, &staker_addr, &owner, &distributor);

        // try to call distribute all as a non-whitelisted user
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: false }).unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify the distribution failed with the expected error
        assert!(dist_res.is_err());
        assert_error(dist_res, "User not whitelisted")
    }

    #[test]
    fn test_distribute_all_when_contract_paused_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let (distributor, _, _) = setup_allocations(&mut app, &staker_addr, &owner);

        // pause the contract
        pause(&mut app, &staker_addr, &owner);

        // try to call distribute all when the contract is paused
        let dist_res = app.execute(
            distributor,
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: false }).unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify the distribution failed with the expected error
        assert!(dist_res.is_err());
        assert_error(dist_res, "Contract is paused")
    }

    #[test]
    fn test_distribute_all_when_user_has_no_allocations_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let distributor = "distributor".into_bech32();
        whitelist_user(&mut app, &staker_addr, &owner, &distributor);

        // try to call distribute all with no allocations
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: false }).unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify the distribution failed with the expected error
        assert!(dist_res.is_err());
        assert_error(dist_res, "No allocations")
    }

    #[test]
    fn test_distribute_all_in_truinj_with_insufficient_truinj_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        set_dist_fee(&mut app, &staker_addr, &owner, 500); // 5%

        let (distributor, first_recipient, second_recipient) =
            setup_allocations(&mut app, &staker_addr, &owner);

        move_days_forward(&mut app, 30);

        // mint to the distributor the exact amount of inj that will be distributed
        let (_, truinj_to_distribute, fees) =
            get_distribution_amounts(&app, &staker_addr, &distributor, None);

        // transfer away some of the truinj so that the distributor has insufficient truinj to distribute and pay the fees
        let initial_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        transfer_truinj(
            &mut app,
            &staker_addr,
            &distributor,
            &"someone else".into_bech32(),
            initial_truinj_balance - truinj_to_distribute - fees + 1,
        );
        let pre_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        assert!(pre_truinj_balance < truinj_to_distribute + fees);

        // try to call distribute all with insufficient truinj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: false }).unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify the distribution failed with the expected error
        assert!(dist_res.is_err());
        assert_error(dist_res, "Insufficient TruINJ balance");

        // verify the distributor and recipients truinj balances were not changed
        assert_eq!(
            query_truinj_balance(&app, &distributor, &staker_addr),
            pre_truinj_balance
        );
        assert_eq!(
            query_truinj_balance(&app, &first_recipient, &staker_addr),
            0
        );
        assert_eq!(
            query_truinj_balance(&app, &second_recipient, &staker_addr),
            0
        );
    }

    #[test]
    fn test_distribute_all_in_inj_with_insufficient_truinj_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        set_dist_fee(&mut app, &staker_addr, &owner, 500); // 5%

        let (distributor, first_recipient, second_recipient) =
            setup_allocations(&mut app, &staker_addr, &owner);

        move_days_forward(&mut app, 30);

        // mint to the distributor the exact amount of inj that will be distributed
        let (inj_to_distribute, _, fees) =
            get_distribution_amounts(&app, &staker_addr, &distributor, None);

        // mint to the distributor the exact amount of inj that will be distributed
        mint_inj(&mut app, &distributor, inj_to_distribute);

        // transfer away some of the inj so that the distributor has insufficient truinj to pay the fees
        let initial_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        transfer_truinj(
            &mut app,
            &staker_addr,
            &distributor,
            &"someone else".into_bech32(),
            initial_truinj_balance - fees + 1,
        );
        let pre_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        assert!(pre_truinj_balance < fees);

        // try to call distribute all with insufficient truinj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: true }).unwrap(),
                funds: vec![coin(inj_to_distribute, INJ)],
            }
            .into(),
        );

        // verify the distribution failed with the expected error
        assert!(dist_res.is_err());
        assert_error(dist_res, "Insufficient TruINJ balance");

        // verify the distributor and recipients truinj balances were not changed
        assert_eq!(
            query_truinj_balance(&app, &distributor, &staker_addr),
            pre_truinj_balance
        );
        assert_eq!(
            query_truinj_balance(&app, &first_recipient, &staker_addr),
            0
        );
        assert_eq!(
            query_truinj_balance(&app, &second_recipient, &staker_addr),
            0
        );
    }

    #[test]
    fn test_distribute_all_in_inj_with_insufficient_inj_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        set_dist_fee(&mut app, &staker_addr, &owner, 500); // 5%

        let (distributor, first_recipient, second_recipient) =
            setup_allocations(&mut app, &staker_addr, &owner);

        move_days_forward(&mut app, 30);

        // mint to the distributor the exact amount of inj that will be distributed
        let (inj_to_distribute, _, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, None);

        // mint to the distributor less inj that it's required to distribute
        let inj_to_attach = inj_to_distribute - 1;
        mint_inj(&mut app, &distributor, inj_to_attach);

        let pre_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);

        // try to call distribute all attaching insufficient inj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeAll { in_inj: true }).unwrap(),
                funds: vec![coin(inj_to_attach, INJ)],
            }
            .into(),
        );

        // verify the distribution failed with the expected error
        assert!(dist_res.is_err());
        assert_error(dist_res, "Insufficient INJ attached");

        // verify the distributor and recipients truinj balances were not changed
        assert_eq!(
            query_truinj_balance(&app, &distributor, &staker_addr),
            pre_truinj_balance
        );
        assert_eq!(
            query_truinj_balance(&app, &first_recipient, &staker_addr),
            0
        );
        assert_eq!(
            query_truinj_balance(&app, &second_recipient, &staker_addr),
            0
        );
    }

    // helper functions

    fn setup_allocations(app: &mut App, staker_addr: &Addr, owner: &Addr) -> (Addr, Addr, Addr) {
        // set up allocations to ttwo different recipients
        let recipients = vec![
            "first-recipient".into_bech32(),
            "second-recipient".into_bech32(),
        ];
        let distributor = setup_allocations_to_recipients(app, owner, &recipients, staker_addr);
        (distributor, recipients[0].clone(), recipients[1].clone())
    }

    fn setup_allocations_to_recipients(
        app: &mut App,
        owner: &Addr,
        recipients: &[Addr],
        staker_addr: &Addr,
    ) -> Addr {
        // set up 2 allocations to different recipients
        let distributor = "distributor".into_bech32();

        for (index, recipient) in recipients.iter().enumerate() {
            let allocation_amount = 100_000 * (index + 1) as u128;
            set_up_test_allocation(
                app,
                owner,
                staker_addr,
                &distributor,
                recipient,
                allocation_amount,
            );
        }

        distributor
    }
}
