pub mod helpers;

#[cfg(test)]
mod validators {

    use cosmwasm_std::{Addr, Attribute, Uint128};
    use cw_multi_test::IntoBech32;

    use helpers::instantiate_staker;
    use injective_staker::{
        msg::{GetValidatorResponse, QueryMsg},
        state::{ValidatorInfo, ValidatorState},
        ContractError,
    };

    use crate::helpers::{
        self, add_validator, add_validator_to_app, disable_validator, enable_validator,
        instantiate_staker_with_min_deposit, mint_inj, stake_to_specific_validator, whitelist_user,
    };

    #[test]
    fn test_add_validator() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let new_validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, default_validator) =
            instantiate_staker(owner.clone(), treasury);

        add_validator_to_app(&mut app, new_validator.to_string());
        add_validator(
            &mut app,
            owner,
            &staking_contract.addr(),
            new_validator.clone(),
        )
        .unwrap();

        let response: GetValidatorResponse = app
            .wrap()
            .query_wasm_smart(staking_contract.addr(), &QueryMsg::GetValidators {})
            .unwrap();
        assert_eq!(response.validators.len(), 2);

        assert_eq!(
            response.validators,
            vec![
                ValidatorInfo {
                    total_staked: Uint128::zero(),
                    state: ValidatorState::ENABLED,
                    addr: default_validator,
                },
                ValidatorInfo {
                    total_staked: Uint128::zero(),
                    state: ValidatorState::ENABLED,
                    addr: new_validator,
                }
            ]
        );
    }

    #[test]
    fn test_get_validators_with_stake() {
        let owner: Addr = "owner".into_bech32();

        let (mut app, staker_addr, default_validator) =
            instantiate_staker_with_min_deposit(owner.clone(), "treasury".into_bech32(), 0);

        // add a second validator
        let second_validator: Addr = "validator".into_bech32();
        add_validator_to_app(&mut app, second_validator.to_string());
        add_validator(
            &mut app,
            owner.clone(),
            &staker_addr,
            second_validator.clone(),
        )
        .unwrap();

        // whitelist a user with tokens
        let alice: Addr = "alice".into_bech32();
        mint_inj(&mut app, &alice, 300_000);
        whitelist_user(&mut app, &staker_addr, &owner, &alice);

        // user stakes to both validators
        let first_stake = Uint128::from(100_000u128);
        stake_to_specific_validator(
            &mut app,
            &alice,
            &staker_addr,
            first_stake.into(),
            &default_validator,
        )
        .unwrap();

        let second_stake = Uint128::from(200_000u128);
        stake_to_specific_validator(
            &mut app,
            &alice,
            &staker_addr,
            second_stake.into(),
            &second_validator,
        )
        .unwrap();

        let response: GetValidatorResponse = app
            .wrap()
            .query_wasm_smart(staker_addr, &QueryMsg::GetValidators {})
            .unwrap();
        assert_eq!(response.validators.len(), 2);

        // verify the validators info
        assert_eq!(
            response.validators,
            vec![
                ValidatorInfo {
                    total_staked: first_stake,
                    state: ValidatorState::ENABLED,
                    addr: default_validator.clone(),
                },
                ValidatorInfo {
                    total_staked: second_stake,
                    state: ValidatorState::ENABLED,
                    addr: second_validator.clone(),
                }
            ]
        );
    }

    #[test]
    fn test_emit_validator_added_event() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, _default_validator) =
            instantiate_staker(owner.clone(), treasury);

        let add_validator_response =
            add_validator(&mut app, owner, &staking_contract.addr(), validator.clone());

        let response = add_validator_response.unwrap();
        let add_validator_event = response.events.last().unwrap();
        assert_eq!(add_validator_event.ty, "wasm-validator_added");

        assert_eq!(
            add_validator_event.attributes.get(1).unwrap(),
            Attribute {
                key: "validator_address".to_string(),
                value: validator.to_string()
            }
        );
    }

    #[test]
    fn test_add_validator_twice_fails() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, _) = instantiate_staker(owner.clone(), treasury);

        add_validator(
            &mut app,
            owner.clone(),
            &staking_contract.addr(),
            validator.clone(),
        )
        .unwrap();
        let err = add_validator(&mut app, owner, &staking_contract.addr(), validator).unwrap_err();

        assert_eq!(
            ContractError::ValidatorAlreadyExists,
            err.downcast().unwrap()
        );
    }

    #[test]
    fn test_add_validator_with_non_owner_fails() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, _) = instantiate_staker(owner, treasury.clone());

        let err =
            add_validator(&mut app, treasury, &staking_contract.addr(), validator).unwrap_err();

        assert_eq!(ContractError::OnlyOwner {}, err.downcast().unwrap());
    }

    #[test]
    fn test_enable_validator_with_non_owner_fails() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, _) = instantiate_staker(owner, treasury.clone());

        let err =
            enable_validator(&mut app, treasury, &staking_contract.addr(), validator).unwrap_err();

        assert_eq!(ContractError::OnlyOwner {}, err.downcast().unwrap());
    }

    #[test]
    fn test_disable_validator_with_non_owner_fails() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, _) = instantiate_staker(owner, treasury.clone());

        let err =
            disable_validator(&mut app, treasury, &staking_contract.addr(), validator).unwrap_err();

        assert_eq!(ContractError::OnlyOwner {}, err.downcast().unwrap());
    }

    #[test]
    fn test_disable_enabled_validator() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, default_validator) =
            instantiate_staker(owner.clone(), treasury);

        add_validator_to_app(&mut app, validator.to_string());
        add_validator(
            &mut app,
            owner.clone(),
            &staking_contract.addr(),
            validator.clone(),
        )
        .unwrap();

        let disable_validator_response =
            disable_validator(&mut app, owner, &staking_contract.addr(), validator.clone())
                .unwrap();

        let response: GetValidatorResponse = app
            .wrap()
            .query_wasm_smart(staking_contract.addr(), &QueryMsg::GetValidators {})
            .unwrap();
        assert_eq!(response.validators.len(), 2);

        assert_eq!(
            response.validators,
            vec![
                ValidatorInfo {
                    total_staked: Uint128::zero(),
                    state: ValidatorState::ENABLED,
                    addr: default_validator,
                },
                ValidatorInfo {
                    total_staked: Uint128::zero(),
                    state: ValidatorState::DISABLED,
                    addr: validator.clone(),
                }
            ]
        );

        // check that the event was emitted
        let disable_validator_event = disable_validator_response.events.last().unwrap();
        assert_eq!(disable_validator_event.ty, "wasm-validator_disabled");

        assert_eq!(
            disable_validator_event.attributes.get(1).unwrap(),
            Attribute {
                key: "validator_address".to_string(),
                value: validator.to_string()
            }
        );
    }

    #[test]
    fn test_enable_disabled_validator() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, default_validator) =
            instantiate_staker(owner.clone(), treasury);

        add_validator_to_app(&mut app, validator.to_string());
        add_validator(
            &mut app,
            owner.clone(),
            &staking_contract.addr(),
            validator.clone(),
        )
        .unwrap();

        disable_validator(
            &mut app,
            owner.clone(),
            &staking_contract.addr(),
            validator.clone(),
        )
        .unwrap();
        let enabled_validator_response =
            enable_validator(&mut app, owner, &staking_contract.addr(), validator.clone()).unwrap();

        let response: GetValidatorResponse = app
            .wrap()
            .query_wasm_smart(staking_contract.addr(), &QueryMsg::GetValidators {})
            .unwrap();
        assert_eq!(response.validators.len(), 2);

        assert_eq!(
            response.validators,
            vec![
                ValidatorInfo {
                    total_staked: Uint128::zero(),
                    state: ValidatorState::ENABLED,
                    addr: default_validator,
                },
                ValidatorInfo {
                    total_staked: Uint128::zero(),
                    state: ValidatorState::ENABLED,
                    addr: validator.clone(),
                }
            ]
        );

        // check that the event was emitted
        let enabled_validator_event = enabled_validator_response.events.last().unwrap();
        assert_eq!(enabled_validator_event.ty, "wasm-validator_enabled");

        assert_eq!(
            enabled_validator_event.attributes.get(1).unwrap(),
            Attribute {
                key: "validator_address".to_string(),
                value: validator.to_string()
            }
        );
    }

    #[test]
    fn test_enable_enabled_validator_fails() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, _) = instantiate_staker(owner.clone(), treasury);

        add_validator(
            &mut app,
            owner.clone(),
            &staking_contract.addr(),
            validator.clone(),
        )
        .unwrap();
        let err =
            enable_validator(&mut app, owner, &staking_contract.addr(), validator).unwrap_err();

        assert_eq!(
            ContractError::ValidatorAlreadyEnabled,
            err.downcast().unwrap()
        );
    }

    #[test]
    fn test_enable_non_existent_validator_fails() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, _) = instantiate_staker(owner.clone(), treasury);

        let err =
            enable_validator(&mut app, owner, &staking_contract.addr(), validator).unwrap_err();

        assert_eq!(
            ContractError::ValidatorDoesNotExist,
            err.downcast().unwrap()
        );
    }

    #[test]
    fn test_disable_non_existent_validator_fails() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, _) = instantiate_staker(owner.clone(), treasury);

        let err =
            disable_validator(&mut app, owner, &staking_contract.addr(), validator).unwrap_err();

        assert_eq!(
            ContractError::ValidatorDoesNotExist,
            err.downcast().unwrap()
        );
    }

    #[test]
    fn test_disable_disabled_validator_fails() {
        let owner: Addr = "owner".into_bech32();
        let treasury: Addr = "treasury".into_bech32();
        let validator: Addr = "validator".into_bech32();

        let (mut app, staking_contract, _) = instantiate_staker(owner.clone(), treasury);

        add_validator(
            &mut app,
            owner.clone(),
            &staking_contract.addr(),
            validator.clone(),
        )
        .unwrap();
        disable_validator(
            &mut app,
            owner.clone(),
            &staking_contract.addr(),
            validator.clone(),
        )
        .unwrap();
        let err =
            disable_validator(&mut app, owner, &staking_contract.addr(), validator).unwrap_err();

        assert_eq!(
            ContractError::ValidatorAlreadyDisabled,
            err.downcast().unwrap()
        );
    }
}
