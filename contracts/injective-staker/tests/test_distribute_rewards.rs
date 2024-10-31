pub mod helpers;

#[cfg(test)]
mod distribute_rewards {

    use cosmwasm_std::{coin, to_json_binary, Decimal, WasmMsg};
    use cw_multi_test::{Executor, IntoBech32, StakingSudo};
    use injective_staker::{msg::ExecuteMsg, INJ};

    use crate::helpers::{
        assert_error, assert_event_with_attributes, clear_whitelist_status,
        get_distribution_amounts, get_share_price_num_denom, get_total_allocated,
        instantiate_staker_with_min_deposit, mint_inj, move_days_forward, pause, query_inj_balance,
        query_truinj_balance, set_dist_fee, set_up_test_allocation, transfer_truinj,
        whitelist_user,
    };

    #[test]
    fn test_distribute_rewards_in_truinj() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), treasury.clone(), 0);

        let dist_fee = 500; // 5%
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);

        // set up an allocation at the current share price
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // accrue rewards
        move_days_forward(&mut app, 30);

        // check balances before distribution
        let pre_distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        let pre_recipient_truinj_balance = query_truinj_balance(&app, &recipient, &staker_addr);
        let pre_treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);
        assert!(pre_distributor_truinj_balance > 0);
        assert_eq!(pre_recipient_truinj_balance, 0);
        assert_eq!(pre_treasury_truinj_balance, 0);

        let (dist_share_price_num, dist_share_price_denom) =
            get_share_price_num_denom(&app, &staker_addr);

        let (inj_to_distribute, truinj_to_distribute, fees) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipient));

        // distribute rewards to recipient at the current share price in truinj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: false,
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        assert!(dist_res.is_ok());

        let distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        let recipient_truinj_balance = query_truinj_balance(&app, &recipient, &staker_addr);
        let treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);

        // verify the distributor balance was reduced by the expected amount of shares
        assert_eq!(
            distributor_truinj_balance,
            pre_distributor_truinj_balance - truinj_to_distribute - fees
        );

        // verify the recipient received the expected amount of shares
        assert_eq!(recipient_truinj_balance, truinj_to_distribute);

        // verify the treasury balance was increased by the expected amount of fees
        assert_eq!(treasury_truinj_balance, fees);

        let (
            total_allocated_amount,
            total_allocated_share_price_num,
            total_allocated_share_price_denom,
        ) = get_total_allocated(&app, &staker_addr, &distributor);

        assert_event_with_attributes(
            &dist_res.unwrap().events,
            "wasm-distributed_rewards",
            vec![
                ("user", distributor).into(),
                ("recipient", recipient).into(),
                ("user_balance", distributor_truinj_balance.to_string()).into(),
                ("recipient_balance", recipient_truinj_balance.to_string()).into(),
                ("treasury_balance", treasury_truinj_balance.to_string()).into(),
                ("fees", fees.to_string()).into(),
                ("shares", truinj_to_distribute.to_string()).into(),
                ("inj_amount", inj_to_distribute.to_string()).into(),
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
            staker_addr,
        );
    }

    #[test]
    fn test_distribute_rewards_after_slashing_distributes_no_rewards() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, validator_addr) =
            instantiate_staker_with_min_deposit(owner.clone(), treasury, 0);

        // set up an allocation at the current share price
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // accrue rewards
        move_days_forward(&mut app, 30);

        // slash the validator by 50%
        app.sudo(cw_multi_test::SudoMsg::Staking(StakingSudo::Slash {
            validator: validator_addr.to_string(),
            percentage: Decimal::percent(50),
        }))
        .unwrap();

        // distribute rewards to recipient in trunj
        let distributor_pre_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        let recipient_pre_truinj_balance = query_truinj_balance(&app, &recipient, &staker_addr);
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: false,
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // verify the distributor and recipient balances did not change
        assert_eq!(
            query_truinj_balance(&app, &distributor, &staker_addr),
            distributor_pre_truinj_balance
        );
        assert_eq!(
            query_truinj_balance(&app, &recipient, &staker_addr),
            recipient_pre_truinj_balance
        );

        // verify no wasm-distributed-rewards event were emitted
        assert_eq!(
            dist_res
                .unwrap()
                .events
                .into_iter()
                .filter(|e| e.ty == "wasm-distributed-rewards")
                .count(),
            0
        );
    }

    #[test]
    fn test_distribute_rewards_in_truinj_when_no_rewards_accrued() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), treasury.clone(), 0);

        let dist_fee = 500; // 5%
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);

        // set up an allocation at the current share price
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // check balances before distribution
        let pre_distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        let pre_recipient_truinj_balance = query_truinj_balance(&app, &recipient, &staker_addr);
        let pre_treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);
        assert!(pre_distributor_truinj_balance > 0);
        assert_eq!(pre_recipient_truinj_balance, 0);
        assert_eq!(pre_treasury_truinj_balance, 0);

        // distribute rewards to recipient in trunj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: false,
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // verify the distributor balance was not reduced
        let distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        assert_eq!(distributor_truinj_balance, pre_distributor_truinj_balance);

        // verify the recipient balance was not increased
        let recipient_truinj_balance = query_truinj_balance(&app, &recipient, &staker_addr);
        assert_eq!(recipient_truinj_balance, 0);

        // verify the treasury balance was not increased
        let treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);
        assert_eq!(treasury_truinj_balance, 0);

        // verify no wasm-distributed_rewards event were emitted
        assert_eq!(
            dist_res
                .unwrap()
                .events
                .into_iter()
                .filter(|e| e.ty == "wasm-distributed_rewards")
                .count(),
            0
        );
    }

    #[test]
    fn test_distribute_rewards_in_truinj_refunds_all_attached_inj() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let dist_fee = 500; // 5%
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);

        // set up an allocation at the current share price
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // accrue rewards
        move_days_forward(&mut app, 30);

        // mint some inj to the distributor
        let attached_inj = 1_000;
        mint_inj(&mut app, &distributor, attached_inj);

        // distribute rewards to recipient in trunj attaching some inj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: false,
                })
                .unwrap(),
                funds: vec![coin(attached_inj, INJ)],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // verify the distributor got the attached inj back
        let distributor_inj_balance = query_inj_balance(&app, &distributor);
        assert_eq!(distributor_inj_balance, attached_inj);
    }

    #[test]
    fn test_distribute_rewards_in_truinj_when_distributor_has_no_truinj_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        // set up an allocation at the current share price
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // the user transfers all their truinj away
        let distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        transfer_truinj(
            &mut app,
            &staker_addr,
            &distributor,
            &"someone else".into_bech32(),
            distributor_truinj_balance,
        );

        let distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        assert_eq!(distributor_truinj_balance, 0);

        // accrue rewards
        move_days_forward(&mut app, 30);

        // try to distribute rewards in trunj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: false,
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify the distribution failed with the expected error message
        assert!(dist_res.is_err());
        assert_error(dist_res, "Insufficient TruINJ balance");
    }

    #[test]
    fn test_distribute_rewards_when_distributor_has_no_allocations_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        // whitelist the user
        let distributor = "distributor".into_bech32();
        whitelist_user(&mut app, &staker_addr, &owner, &distributor);

        // try to distribute rewards to a recipient
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: "recipient".into_bech32().into_string(),
                    in_inj: false,
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify the distribution failed with the expected error message
        assert!(dist_res.is_err());
        assert_error(dist_res, "No allocations");
    }

    #[test]
    fn test_distribute_rewards_when_distributor_has_no_allocation_to_recipient_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        // set up an allocation
        let distributor = "distributor".into_bech32();
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &"recipient".into_bech32(),
            100_000,
        );

        // try to distribute rewards to a different recipient
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: "someone else".into_bech32().into_string(),
                    in_inj: false,
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify the distribution failed with the expected error message
        assert!(dist_res.is_err());
        assert_error(dist_res, "No Allocation to recipient");
    }

    #[test]
    fn test_distribute_rewards_when_user_not_whitelisted_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        // set up an allocation
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            100_000,
        );

        // remove user from whitelist
        clear_whitelist_status(&mut app, &staker_addr, &owner, &distributor);

        // try to distribute rewards
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: false,
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify the distribution failed with the expected error message
        assert!(dist_res.is_err());
        assert_error(dist_res, "User not whitelisted");
    }

    #[test]
    fn test_distribute_rewards_when_contract_is_paused_fails() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), treasury, 0);
        // set up an allocation at the current share price
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // pause the contract
        pause(&mut app, &staker_addr, &owner);

        // distribute rewards to recipient in trunj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: false,
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // verify the distribution failed with the expected error message
        assert!(dist_res.is_err());
        assert_error(dist_res, "Contract is paused");
    }

    #[test]
    fn test_distribute_rewards_in_inj() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), treasury.clone(), 0);

        let dist_fee = 500; // 5%
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);

        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // accrue rewards
        move_days_forward(&mut app, 365);

        let pre_distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        let (dist_share_price_num, dist_share_price_denom) =
            get_share_price_num_denom(&app, &staker_addr);

        let (inj_to_distribute, truinj_to_distribute, fees) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipient));

        // mint to the distributor the exact amount of inj that will be distributed
        mint_inj(&mut app, &distributor, inj_to_distribute);

        // distribute rewards to recipient in inj attaching the exact amount of inj to distribute
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: true,
                })
                .unwrap(),
                funds: vec![coin(inj_to_distribute, INJ)],
            }
            .into(),
        );

        assert!(dist_res.is_ok());

        let distributor_truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        let recipient_truinj_balance = query_truinj_balance(&app, &recipient, &staker_addr);
        let recipient_inj_balance = query_inj_balance(&app, &recipient);
        let treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);

        // verify the distributor balance was reduced by the fees paid to the treasury
        assert_eq!(
            distributor_truinj_balance,
            pre_distributor_truinj_balance - fees
        );

        // verify the recipient received the expected amount of inj
        assert_eq!(recipient_inj_balance, inj_to_distribute);

        // verify the treasury received the expected amount of fees
        assert_eq!(treasury_truinj_balance, fees);

        let (
            total_allocated_amount,
            total_allocated_share_price_num,
            total_allocated_share_price_denom,
        ) = get_total_allocated(&app, &staker_addr, &distributor);

        assert_event_with_attributes(
            &dist_res.unwrap().events,
            "wasm-distributed_rewards",
            vec![
                ("user", distributor).into(),
                ("recipient", recipient).into(),
                ("user_balance", distributor_truinj_balance.to_string()).into(),
                ("recipient_balance", recipient_truinj_balance.to_string()).into(),
                ("treasury_balance", treasury_truinj_balance.to_string()).into(),
                ("fees", fees.to_string()).into(),
                ("shares", truinj_to_distribute.to_string()).into(),
                ("inj_amount", inj_to_distribute.to_string()).into(),
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
            staker_addr,
        );
    }

    #[test]
    fn test_distribute_rewards_in_inj_when_no_rewards_accrued() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), treasury.clone(), 0);

        let dist_fee = 500; // 5%
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);

        // set up an allocation at the current share price
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // distribute rewards to recipient in inj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: true,
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // verify the recipient inj balance was not increased
        let recipient_inj_balance = query_inj_balance(&app, &recipient);
        assert_eq!(recipient_inj_balance, 0);

        // verify the treasury truinj balance was not increased
        let treasury_truinj_balance = query_truinj_balance(&app, &treasury, &staker_addr);
        assert_eq!(treasury_truinj_balance, 0);

        // verify no wasm-distributed_rewards event were emitted
        assert_eq!(
            dist_res
                .unwrap()
                .events
                .into_iter()
                .filter(|e| e.ty == "wasm-distributed_rewards")
                .count(),
            0
        );
    }

    #[test]
    fn test_distribute_rewards_in_inj_refunds_unused_attached_deposit() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let dist_fee = 500; // 5%
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);

        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // accrue rewards
        move_days_forward(&mut app, 30);

        // mint to the distributor the exact amount of inj that will be distributed
        let (inj_to_distribute, _, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipient));

        // mint to the distributor more inj that will be distributed
        let excess_inj = 1000;
        mint_inj(&mut app, &distributor, inj_to_distribute + excess_inj);
        let pre_distributor_inj_balance = query_inj_balance(&app, &distributor);
        assert_eq!(pre_distributor_inj_balance, inj_to_distribute + excess_inj);

        // distribute rewards attaching the inj in excess
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: true,
                })
                .unwrap(),
                funds: vec![coin(inj_to_distribute + excess_inj, INJ)],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // verify the distributor was refunded the attached inj in excess
        let distributor_inj_balance = query_inj_balance(&app, &distributor);
        assert_eq!(distributor_inj_balance, excess_inj);
    }

    #[test]
    fn test_distribute_rewards_in_inj_with_no_distribution_refunds_all_attached_deposit() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // verify there are no rewards to distribute
        let (inj_to_distribute, truinj_to_distribute, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipient));

        assert_eq!(inj_to_distribute, 0);
        assert_eq!(truinj_to_distribute, 0);

        // mint some inj to the distributor
        let inj_amount = 1000;
        mint_inj(&mut app, &distributor, inj_amount);
        let pre_distributor_inj_balance = query_inj_balance(&app, &distributor);
        assert_eq!(pre_distributor_inj_balance, inj_amount);

        // distribute rewards attaching the minted inj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: true,
                })
                .unwrap(),
                funds: vec![coin(inj_amount, INJ)],
            }
            .into(),
        );
        assert!(dist_res.is_ok());

        // verify the distributor was refunded the full attached inj amount
        let distributor_inj_balance = query_inj_balance(&app, &distributor);
        assert_eq!(distributor_inj_balance, inj_amount);
    }

    #[test]
    fn test_distribute_rewards_in_inj_with_insufficient_inj_attached_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let dist_fee = 500; // 5%
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);

        // set up an allocation
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // accrue rewards
        move_days_forward(&mut app, 365);

        // mint to the distributor less inj that will be distributed
        let (inj_to_distribute, _, _) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipient));
        mint_inj(&mut app, &distributor, inj_to_distribute - 1);

        // try to distribute rewards attaching less inj than required
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: true,
                })
                .unwrap(),
                funds: vec![coin(inj_to_distribute - 1, INJ)],
            }
            .into(),
        );

        // verify the distribution failed with the expected error message
        assert!(dist_res.is_err());
        assert_error(dist_res, "Insufficient INJ attached");
    }

    #[test]
    fn test_distribute_rewards_in_inj_with_insufficient_truinj_fails() {
        let owner = "owner".into_bech32();
        let (mut app, staker_addr, _) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        let dist_fee = 500; // 5%
        set_dist_fee(&mut app, &staker_addr, &owner, dist_fee);

        // set up an allocation
        let distributor = "distributor".into_bech32();
        let recipient = "recipient".into_bech32();
        let allocation_amount = 100_000;
        set_up_test_allocation(
            &mut app,
            &owner,
            &staker_addr,
            &distributor,
            &recipient,
            allocation_amount,
        );

        // accrue rewards
        move_days_forward(&mut app, 365);

        // mint to the distributor the exact amount of inj that will be distributed
        let (inj_to_distribute, _, fees) =
            get_distribution_amounts(&app, &staker_addr, &distributor, Some(&recipient));
        mint_inj(&mut app, &distributor, inj_to_distribute);

        // transfer away some of the truinj so that the distributor has insufficient truinj to pay the fees
        let truinj_balance = query_truinj_balance(&app, &distributor, &staker_addr);
        transfer_truinj(
            &mut app,
            &staker_addr,
            &distributor,
            &"someone else".into_bech32(),
            truinj_balance - fees + 1,
        );
        assert!(query_truinj_balance(&app, &distributor, &staker_addr) < fees);

        // try to distribute rewards having insufficient truinj
        let dist_res = app.execute(
            distributor.clone(),
            WasmMsg::Execute {
                contract_addr: staker_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::DistributeRewards {
                    recipient: recipient.to_string(),
                    in_inj: true,
                })
                .unwrap(),
                funds: vec![coin(inj_to_distribute, INJ)],
            }
            .into(),
        );

        // verify the distribution failed with the expected error message
        assert!(dist_res.is_err());
        assert_error(dist_res, "Insufficient TruINJ balance");
    }
}
