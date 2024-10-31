pub mod helpers;

#[cfg(test)]
mod whitelist {

    use cosmwasm_std::Addr;
    use cw_multi_test::{Executor, IntoBech32};
    use std::vec;

    use helpers::instantiate_staker;
    use injective_staker::{msg::ExecuteMsg, state::UserStatus};

    use crate::helpers::{
        self, add_agent, assert_error, assert_event_with_attributes, blacklist_user,
        is_user_blacklisted, is_user_whitelisted, query_is_agent, query_user_status,
        wasm_execute_msg, whitelist_user,
    };

    #[test]
    fn test_owner_is_agent() {
        let owner: Addr = "owner".into_bech32();
        let (app, staker_contract, _) = instantiate_staker(owner.clone(), "treasury".into_bech32());

        // verify that the owner is an agent
        let is_agent = query_is_agent(&app, &owner, &staker_contract);
        assert!(is_agent);
    }

    // Add Agent Tests //

    #[test]
    fn test_add_agent() {
        let owner: Addr = "owner".into_bech32();
        let new_agent: Addr = "agent".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // execute add agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::AddAgent {
                agent: new_agent.to_string(),
            },
        );

        let response = app.execute(owner, msg.into());
        assert!(response.is_ok());

        // verify that the new agent was added
        let is_agent = query_is_agent(&app, &new_agent, &staker_contract);
        assert!(is_agent);
    }

    #[test]
    fn test_add_agent_with_invalid_address_fails() {
        let owner: Addr = "owner".into_bech32();
        let new_agent: Addr = Addr::unchecked("agent");

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // execute add agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::AddAgent {
                agent: new_agent.to_string(),
            },
        );

        let response = app.execute(owner, msg.into());
        assert_error(response, "Generic error: Error decoding bech32");
    }

    #[test]
    fn test_add_agent_emits_event() {
        let owner: Addr = "owner".into_bech32();
        let new_agent: Addr = "agent".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // execute add agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::AddAgent {
                agent: new_agent.to_string(),
            },
        );

        let response = app.execute(owner, msg.into());
        assert!(response.is_ok());

        // verify that the event was emitted
        assert_event_with_attributes(
            &response.unwrap().events,
            "wasm-agent_added",
            vec![("new_agent", new_agent.to_string()).into()],
            staker_contract,
        );
    }

    #[test]
    fn test_add_agent_when_caller_not_agent_fails() {
        let owner: Addr = "owner".into_bech32();
        let user: Addr = "user".into_bech32();
        let new_agent: Addr = "agent".into_bech32();

        let (mut app, staker_contract, _) = instantiate_staker(owner, "treasury".into_bech32());

        // execute add agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::AddAgent {
                agent: new_agent.to_string(),
            },
        );

        let response = app.execute(user, msg.into());
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Caller is not an agent");
    }

    #[test]
    fn test_add_agent_when_adding_owner_fails() {
        let owner: Addr = "owner".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // execute add agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::AddAgent {
                agent: owner.to_string(),
            },
        );

        let response = app.execute(owner, msg.into());
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Ower cannot be added");
    }

    #[test]
    fn test_add_agent_when_adding_existing_agent_fails() {
        let owner: Addr = "owner".into_bech32();
        let new_agent: Addr = "agent".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // add the Agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::AddAgent {
                agent: new_agent.to_string(),
            },
        );

        let _ = app.execute(owner.clone(), msg.clone().into());

        // try to add the agent again
        let response = app.execute(owner, msg.into());

        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Agent already exists");
    }

    // Remove Agent Tests //

    #[test]
    fn test_remove_agent() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // add an agent
        add_agent(&mut app, &staker_contract, &owner, &agent);

        // execute remove agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::RemoveAgent {
                agent: agent.to_string(),
            },
        );

        let response = app.execute(owner.clone(), msg.into());
        assert!(response.is_ok());

        // verify that the new agent was removed
        let is_agent = query_is_agent(&app, &agent, &staker_contract);
        assert!(!is_agent);
    }

    #[test]
    fn test_remove_agent_emits_event() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // add an agent
        add_agent(&mut app, &staker_contract, &owner, &agent);

        // execute remove agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::RemoveAgent {
                agent: agent.to_string(),
            },
        );

        let response = app.execute(owner.clone(), msg.into());
        assert!(response.is_ok());

        // verify that the event was emitted
        assert_event_with_attributes(
            &response.unwrap().events,
            "wasm-agent_removed",
            vec![("removed_agent", agent.to_string()).into()],
            staker_contract,
        );
    }

    #[test]
    fn test_remove_agent_when_caller_not_agent_fails() {
        let owner: Addr = "owner".into_bech32();
        let user: Addr = "user".into_bech32();
        let agent: Addr = "agent".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // add an agent
        add_agent(&mut app, &staker_contract, &owner, &agent);

        // execute remove agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::RemoveAgent {
                agent: agent.to_string(),
            },
        );

        let response = app.execute(user, msg.into());
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Caller is not an agent");
    }

    #[test]
    fn test_remove_agent_when_removing_owner_fails() {
        let owner: Addr = "owner".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // execute remove agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::RemoveAgent {
                agent: owner.to_string(),
            },
        );

        let response = app.execute(owner, msg.into());
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Owner cannot be removed");
    }

    #[test]
    fn test_remove_agent_when_agent_does_not_exist_fails() {
        let owner: Addr = "owner".into_bech32();
        let non_agent: Addr = "non-agent".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // execute remove agent
        let msg = wasm_execute_msg(
            &staker_contract,
            &ExecuteMsg::RemoveAgent {
                agent: non_agent.to_string(),
            },
        );

        let response = app.execute(owner, msg.into());
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Agent does not exist");
    }

    // Add User to Whitelist Tests //

    #[test]
    fn test_add_user_to_whitelist() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_agent(&mut app, &staker_contract, &owner, &agent);

        // add the user to the whitelist
        let user: Addr = "user".into_bech32();
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToWhitelist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_ok());

        // verify that the user was whitelisted
        let is_whitelisted = is_user_whitelisted(&app, &user, &staker_contract);
        assert!(is_whitelisted);
    }

    #[test]
    fn test_add_user_to_whitelist_with_invalid_address_fails() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_agent(&mut app, &staker_contract, &owner, &agent);

        // add the user to the whitelist
        let user: Addr = Addr::unchecked("user");
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToWhitelist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert_error(response, "Generic error: Error decoding bech32");
    }

    #[test]
    fn test_add_user_to_whitelist_emits_event() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_agent(&mut app, &staker_contract, &owner, &agent);

        // add the user to the whitelist
        let user: Addr = "user".into_bech32();
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToWhitelist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_ok());

        // verify that the event was emitted
        assert_event_with_attributes(
            &response.unwrap().events,
            "wasm-whitelisting_status_changed",
            vec![
                ("user", user.to_string()).into(),
                ("old_status", "no_status").into(),
                ("new_status", "whitelisted").into(),
            ],
            staker_contract,
        );
    }

    #[test]
    fn test_add_user_to_whitelist_when_caller_is_not_agent_fails() {
        let owner: Addr = "owner".into_bech32();
        let non_agent: Addr = "non-agent".into_bech32();
        let (mut app, staker_contract, _) = instantiate_staker(owner, "treasury".into_bech32());

        // add the user to the whitelist
        let user: Addr = "user".into_bech32();
        let response = app.execute(
            non_agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToWhitelist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Caller is not an agent");

        // verify that the user was not whitelisted
        let is_whitelisted = is_user_whitelisted(&app, &user, &staker_contract);
        assert!(!is_whitelisted);
    }

    #[test]
    fn test_add_user_to_whitelist_when_user_already_whitelisted_fails() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let user: Addr = "user".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_agent(&mut app, &staker_contract, &owner, &agent);

        // add the user to the whitelist
        whitelist_user(&mut app, &staker_contract, &agent, &user);

        // add the user to the whitelist again
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToWhitelist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "User already whitelisted");
    }

    // Add User to Blacklist Tests //

    #[test]
    fn test_add_user_to_blacklist() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let user: Addr = "user".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_agent(&mut app, &staker_contract, &owner, &agent);

        // add the user to the blacklist
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToBlacklist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_ok());

        // verify that the user was blacklisted
        let is_wblacklisted = is_user_blacklisted(&app, &user, &staker_contract);
        assert!(is_wblacklisted);
    }

    #[test]
    fn test_add_user_to_blacklist_emits_event() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let user: Addr = "user".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_agent(&mut app, &staker_contract, &owner, &agent);

        // add the user to the blacklist
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToBlacklist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_ok());

        // verify that the event was emitted
        assert_event_with_attributes(
            &response.unwrap().events,
            "wasm-whitelisting_status_changed",
            vec![
                ("user", user.to_string()).into(),
                ("old_status", "no_status").into(),
                ("new_status", "blacklisted").into(),
            ],
            staker_contract,
        );
    }

    #[test]
    fn test_add_user_to_blacklist_with_invalid_address_fails() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_agent(&mut app, &staker_contract, &owner, &agent);

        // add the user to the blacklist
        let user: Addr = Addr::unchecked("user");
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToBlacklist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert_error(response, "Generic error: Error decoding bech32");
    }

    #[test]
    fn test_add_user_to_backlist_removes_user_from_whitelist() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let user: Addr = "user".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_agent(&mut app, &staker_contract, &owner, &agent);

        // add the user to the whitelist
        whitelist_user(&mut app, &staker_contract, &agent, &user);

        // add the user to the blacklist
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToBlacklist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_ok());

        // verify that the user was removed from the whitelist
        let is_whitelisted = is_user_whitelisted(&app, &user, &staker_contract);
        assert!(!is_whitelisted);
    }

    #[test]
    fn test_add_user_to_blacklist_when_caller_is_not_agent_fails() {
        let owner: Addr = "owner".into_bech32();
        let non_agent: Addr = "non-agent".into_bech32();
        let user: Addr = "user".into_bech32();
        let (mut app, staker_contract, _) = instantiate_staker(owner, "treasury".into_bech32());

        // add the user to the blacklist
        let response = app.execute(
            non_agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToBlacklist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Caller is not an agent");

        // verify that the user was not blacklisted
        let is_blacklisted = is_user_blacklisted(&app, &user, &staker_contract);
        assert!(!is_blacklisted);
    }

    #[test]
    fn test_add_user_to_blacklist_when_user_already_blacklisted_fails() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let user: Addr = "user".into_bech32();
        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_agent(&mut app, &staker_contract, &owner, &agent);

        // add the user to the blacklist
        blacklist_user(&mut app, &staker_contract, &agent, &user);

        // add the user to the blacklist again
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::AddUserToBlacklist {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "User already blacklisted");
    }

    // Clear User Status Tests //

    #[test]
    fn test_clear_user_status() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let user: Addr = "user".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        add_agent(&mut app, &staker_contract, &owner, &agent);
        whitelist_user(&mut app, &staker_contract, &owner, &user);

        // clear the user whitelist status
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::ClearUserStatus {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_ok());

        // verify that the user status was cleared
        let user_status = query_user_status(&app, &user, &staker_contract);
        assert_eq!(user_status, UserStatus::NoStatus);
        assert_eq!(user_status.to_string(), "no_status");
    }

    #[test]
    fn test_clear_user_status_emits_event() {
        let owner: Addr = "owner".into_bech32();
        let agent: Addr = "agent".into_bech32();
        let user: Addr = "user".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // whitelist the user
        add_agent(&mut app, &staker_contract, &owner, &agent);
        whitelist_user(&mut app, &staker_contract, &owner, &user);

        // clear the user whitelist status
        let response = app.execute(
            agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::ClearUserStatus {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_ok());

        // verify that the event was emitted
        assert_event_with_attributes(
            &response.unwrap().events,
            "wasm-whitelisting_status_changed",
            vec![
                ("user", user.to_string()).into(),
                ("old_status", "whitelisted").into(),
                ("new_status", "no_status").into(),
            ],
            staker_contract,
        );
    }

    #[test]
    fn test_clear_user_status_when_not_agent_fails() {
        let owner: Addr = "owner".into_bech32();
        let non_agent: Addr = "non-agent".into_bech32();
        let user: Addr = "user".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        whitelist_user(&mut app, &staker_contract, &owner, &user);

        // clear the user whitelist status
        let response = app.execute(
            non_agent,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::ClearUserStatus {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "Caller is not an agent");
    }

    #[test]
    fn test_clear_user_status_when_status_is_cleared_fails() {
        let owner: Addr = "owner".into_bech32();
        let user: Addr = "user".into_bech32();

        let (mut app, staker_contract, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // clear the user whitelist status
        let response = app.execute(
            owner,
            wasm_execute_msg(
                &staker_contract,
                &ExecuteMsg::ClearUserStatus {
                    user: user.to_string(),
                },
            )
            .into(),
        );
        assert!(response.is_err());

        // verify the error message
        assert_error(response, "User status already cleared");
    }
}
