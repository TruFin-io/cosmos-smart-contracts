pub mod helpers;

#[cfg(test)]
mod pausability {
    use crate::helpers::{
        self, assert_error, assert_event_with_attributes, pause, query_staker_info,
        wasm_execute_msg,
    };
    use cw_multi_test::{Executor, IntoBech32};
    use helpers::instantiate_staker;
    use injective_staker::msg::ExecuteMsg;

    // Pause Tests //

    #[test]
    fn test_pause() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();

        let (mut app, staker_contract, _) = instantiate_staker(owner.clone(), treasury);

        // verify the contract is not paused
        let staker_info = query_staker_info(&app, &staker_contract.addr());
        assert!(!staker_info.is_paused);

        // pause contract
        let response = app.execute(
            owner,
            wasm_execute_msg(&staker_contract.addr(), &ExecuteMsg::Pause).into(),
        );
        assert!(response.is_ok());

        // verify the contract is paused
        let staker_info = query_staker_info(&app, &staker_contract.addr());
        assert!(staker_info.is_paused);
    }

    #[test]
    fn test_pause_emits_event() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();

        let (mut app, staker_contract, _) = instantiate_staker(owner.clone(), treasury);

        // pause contract
        let response = app.execute(
            owner,
            wasm_execute_msg(&staker_contract.addr(), &ExecuteMsg::Pause).into(),
        );
        assert!(response.is_ok());

        // verify that the event was emitted
        assert_event_with_attributes(
            &response.unwrap().events,
            "wasm-paused",
            vec![],
            staker_contract.addr(),
        );
    }

    #[test]
    fn test_pause_with_non_owner_fails() {
        let owner = "owner".into_bech32();
        let non_onwer = "non-owner".into_bech32();
        let treasury = "treasury".into_bech32();

        let (mut app, staker_contract, _) = instantiate_staker(owner, treasury);

        // try to pause the contract
        let response = app.execute(
            non_onwer,
            wasm_execute_msg(&staker_contract.addr(), &ExecuteMsg::Pause).into(),
        );
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Only the owner can call this method");
    }

    #[test]
    fn test_pause_with_paused_contract_fails() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();

        let (mut app, staker_contract, _) = instantiate_staker(owner.clone(), treasury);

        // pause the contract
        pause(&mut app, &staker_contract.addr(), &owner);
        let staker_info = query_staker_info(&app, &staker_contract.addr());
        assert!(staker_info.is_paused);

        // try to pause contract again
        let response = app.execute(
            owner,
            wasm_execute_msg(&staker_contract.addr(), &ExecuteMsg::Pause).into(),
        );
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Contract is paused")
    }

    // Unpause tests //

    #[test]
    fn test_unpause() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();

        let (mut app, staker_contract, _) = instantiate_staker(owner.clone(), treasury);

        // pause the contract
        pause(&mut app, &staker_contract.addr(), &owner);
        let staker_info = query_staker_info(&app, &staker_contract.addr());
        assert!(staker_info.is_paused);

        // unpause the contract
        let response = app.execute(
            owner,
            wasm_execute_msg(&staker_contract.addr(), &ExecuteMsg::Unpause).into(),
        );
        assert!(response.is_ok());

        // verify the contract is unpaused
        let staker_info = query_staker_info(&app, &staker_contract.addr());
        assert!(!staker_info.is_paused);
    }

    #[test]
    fn test_unpause_emits_event() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();

        let (mut app, staker_contract, _) = instantiate_staker(owner.clone(), treasury);
        pause(&mut app, &staker_contract.addr(), &owner);

        // unpause the contract
        let response = app.execute(
            owner,
            wasm_execute_msg(&staker_contract.addr(), &ExecuteMsg::Unpause).into(),
        );
        assert!(response.is_ok());

        // verify that the event was emitted
        assert_event_with_attributes(
            &response.unwrap().events,
            "wasm-unpaused",
            vec![],
            staker_contract.addr(),
        );
    }

    #[test]
    fn test_unpause_with_non_owner_fails() {
        let owner = "owner".into_bech32();
        let non_onwer = "non-owner".into_bech32();
        let treasury = "treasury".into_bech32();

        let (mut app, staker_contract, _) = instantiate_staker(owner.clone(), treasury);

        // pause the contract
        pause(&mut app, &staker_contract.addr(), &owner);

        // try to unpause the contract
        let response = app.execute(
            non_onwer,
            wasm_execute_msg(&staker_contract.addr(), &ExecuteMsg::Pause).into(),
        );
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Only the owner can call this method");
    }

    #[test]
    fn test_unpause_with_unpaused_contract_fails() {
        let owner = "owner".into_bech32();
        let treasury = "treasury".into_bech32();

        let (mut app, staker_contract, _) = instantiate_staker(owner.clone(), treasury);

        // verify the contract is not paused
        let staker_info = query_staker_info(&app, &staker_contract.addr());
        assert!(!staker_info.is_paused);

        // try to unpause contract
        let response = app.execute(
            owner,
            wasm_execute_msg(&staker_contract.addr(), &ExecuteMsg::Unpause).into(),
        );
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Contract is not paused")
    }
}
