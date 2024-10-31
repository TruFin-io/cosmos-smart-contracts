pub mod helpers;

#[cfg(test)]
mod ownership {
    use cosmwasm_std::{Addr, Attribute};
    use cw_multi_test::{Executor, IntoBech32};
    use injective_staker::msg::{ExecuteMsg, GetIsOwnerResponse, QueryMsg};

    use crate::helpers::{assert_error, instantiate_staker, wasm_execute_msg};

    #[test]
    fn test_set_pending_owner() {
        let owner: Addr = "owner".into_bech32();
        let new_owner: Addr = "new_owner".into_bech32();

        let (mut app, staker_addr, _) = instantiate_staker(owner.clone(), "treasury".into_bech32());

        let msg = ExecuteMsg::SetPendingOwner {
            new_owner: new_owner.to_string(),
        };
        let wasm_msg = wasm_execute_msg(&staker_addr, &msg);

        let response = app.execute(owner.clone(), wasm_msg.into()).unwrap();

        let events = &response.events.last().unwrap();
        assert_eq!("wasm-set_pending_owner", events.ty);
        assert_eq!(
            events.attributes.get(1).unwrap(),
            Attribute {
                key: "current_owner".to_string(),
                value: owner.to_string()
            }
        );

        assert_eq!(
            events.attributes.get(2).unwrap(),
            Attribute {
                key: "pending_owner".to_string(),
                value: new_owner.to_string()
            }
        );
    }

    #[test]
    fn test_set_pending_owner_twice() {
        let owner: Addr = "owner".into_bech32();
        let new_owner: Addr = "new_owner".into_bech32();
        let second_owner: Addr = "second_owner".into_bech32();

        let (mut app, staker_addr, _) = instantiate_staker(owner.clone(), "treasury".into_bech32());

        // First change owner
        let first_msg = ExecuteMsg::SetPendingOwner {
            new_owner: new_owner.to_string(),
        };
        let wasm_msg = wasm_execute_msg(&staker_addr, &first_msg);

        let response = app.execute(owner.clone(), wasm_msg.into());
        assert!(response.is_ok());

        // Second change owner
        let second_msg = ExecuteMsg::SetPendingOwner {
            new_owner: second_owner.to_string(),
        };
        let wasm_msg = wasm_execute_msg(&staker_addr, &second_msg);

        let second_response = app.execute(owner, wasm_msg.into());
        assert!(second_response.is_ok());
    }

    #[test]
    fn test_set_pending_owner_called_by_non_owner_fails() {
        let owner: Addr = "owner".into_bech32();
        let new_owner: Addr = "new_owner".into_bech32();

        let (mut app, staker_addr, _) = instantiate_staker(owner, "treasury".into_bech32());

        let msg = ExecuteMsg::SetPendingOwner {
            new_owner: new_owner.to_string(),
        };
        let wasm_msg = wasm_execute_msg(&staker_addr, &msg);

        let response = app.execute(new_owner, wasm_msg.into());
        assert!(response.is_err());

        assert_error(response, "Only the owner can call this method");
    }

    #[test]
    fn test_claim_ownership() {
        let owner: Addr = "owner".into_bech32();
        let new_owner: Addr = "new_owner".into_bech32();

        let (mut app, staker_addr, _) = instantiate_staker(owner.clone(), "treasury".into_bech32());

        // set pending owner
        let msg = ExecuteMsg::SetPendingOwner {
            new_owner: new_owner.to_string(),
        };
        let wasm_msg = wasm_execute_msg(&staker_addr, &msg);

        app.execute(owner.clone(), wasm_msg.into()).unwrap();

        // claim ownership
        let msg = ExecuteMsg::ClaimOwnership {};
        let wasm_msg = wasm_execute_msg(&staker_addr, &msg);

        let response = app.execute(new_owner.clone(), wasm_msg.into()).unwrap();

        let events = &response.events.last().unwrap();
        assert_eq!("wasm-claimed_ownership", events.ty);
        assert_eq!(
            events.attributes.get(1).unwrap(),
            Attribute {
                key: "new_owner".to_string(),
                value: new_owner.to_string()
            }
        );
        assert_eq!(
            events.attributes.get(2).unwrap(),
            Attribute {
                key: "old_owner".to_string(),
                value: owner.to_string()
            }
        );

        // check if new owner has been set
        let is_owner_response: GetIsOwnerResponse = app
            .wrap()
            .query_wasm_smart(
                staker_addr,
                &QueryMsg::IsOwner {
                    addr: new_owner.to_string(),
                },
            )
            .unwrap();
        assert!(is_owner_response.is_owner);
    }

    #[test]
    fn test_claim_ownership_when_no_pending_owner_is_set_fails() {
        let owner: Addr = "owner".into_bech32();
        let new_owner: Addr = "new_owner".into_bech32();

        let (mut app, staker_addr, _) = instantiate_staker(owner, "treasury".into_bech32());

        // claim ownership
        let msg = ExecuteMsg::ClaimOwnership {};
        let wasm_msg = wasm_execute_msg(&staker_addr, &msg);

        let response = app.execute(new_owner, wasm_msg.into());
        assert!(response.is_err());

        assert_error(response, "There is no pending owner set");
    }

    #[test]
    fn test_claim_ownership_with_non_pending_owner_fails() {
        let owner: Addr = "owner".into_bech32();
        let new_owner: Addr = "new_owner".into_bech32();
        let anyone: Addr = "anyone".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker(owner.clone(), "treasury".into_bech32());

        // set pending owner
        let msg = ExecuteMsg::SetPendingOwner {
            new_owner: new_owner.to_string(),
        };
        let wasm_msg = wasm_execute_msg(&staker_addr, &msg);

        app.execute(owner, wasm_msg.into()).unwrap();

        // claim ownership
        let msg = ExecuteMsg::ClaimOwnership {};
        let wasm_msg = wasm_execute_msg(&staker_addr, &msg);

        let response = app.execute(anyone, wasm_msg.into());
        assert!(response.is_err());

        assert_error(response, "Only the pending owner can call this method");
    }
}
