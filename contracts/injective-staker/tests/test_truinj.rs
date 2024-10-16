pub mod helpers;

#[cfg(test)]
mod truinj {

    use cosmwasm_std::{to_json_binary, Addr, WasmMsg};
    use cw20::{LogoInfo, MarketingInfoResponse, TokenInfoResponse};
    use cw_multi_test::{Executor, IntoBech32};
    use helpers::{contract_wrapper, mint_inj, mock_app_with_validator, query_truinj_balance};
    use injective_staker::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

    use crate::helpers::{self, stake, whitelist_user};

    #[test]
    fn test_stake_mints_truinj() {
        let (mut app, validator_addr) = mock_app_with_validator();
        let code_id = app.store_code(contract_wrapper());

        // Instantiate the contract
        let owner = "owner".into_bech32();
        let msg = InstantiateMsg {
            treasury: "treasury".into_bech32(),
            default_validator: validator_addr,
        };

        let contract_addr = app
            .instantiate_contract(code_id, owner.clone(), &msg, &[], "staker-contract", None)
            .unwrap();

        let anyone: Addr = "anyone".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        // query the balance of 'anyone'
        let sender_balance = query_truinj_balance(&app, &anyone, &contract_addr);

        assert_eq!(sender_balance, inj_to_mint);
    }

    #[test]
    fn test_users_can_transfer_truinj() {
        let (mut app, validator_addr) = mock_app_with_validator();
        let code_id = app.store_code(contract_wrapper());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // instantiate the contract
        let owner = "owner".into_bech32();
        let msg = InstantiateMsg {
            treasury: "treasury".into_bech32(),
            default_validator: validator_addr,
        };
        let contract_addr = app
            .instantiate_contract(code_id, owner.clone(), &msg, &[], "staker-contract", None)
            .unwrap();

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        let msg = ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount: (inj_to_mint / 2).into(),
        };
        let cosmos_msg = WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&msg).unwrap(),
            funds: vec![],
        };
        app.execute(anyone.clone(), cosmos_msg.into()).unwrap();

        // query the balance of 'anyone'
        let sender_balance = query_truinj_balance(&app, &anyone, &contract_addr);

        assert_eq!(sender_balance, inj_to_mint / 2);

        // query the balance of the recipient
        let recipient_balance = query_truinj_balance(&app, &recipient, &contract_addr);

        assert_eq!(recipient_balance, inj_to_mint / 2);
    }

    #[test]
    fn test_can_retrieve_token_info() {
        let (mut app, validator_addr) = mock_app_with_validator();
        let code_id = app.store_code(contract_wrapper());

        // mint INJ tokens to the 'anyone' user
        let anyone: Addr = "anyone".into_bech32();
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

        // instantiate the contract
        let owner = "owner".into_bech32();
        let msg = InstantiateMsg {
            treasury: "treasury".into_bech32(),
            default_validator: validator_addr,
        };
        let contract_addr = app
            .instantiate_contract(code_id, owner.clone(), &msg, &[], "staker-contract", None)
            .unwrap();

        // whitelist user
        whitelist_user(&mut app, &contract_addr, &owner, &anyone);

        // execute stake
        stake(&mut app, &anyone, &contract_addr, inj_to_mint).unwrap();

        // query the Token Info
        let sender_balance: TokenInfoResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::TokenInfo {})
            .unwrap();

        assert_eq!(
            sender_balance,
            TokenInfoResponse {
                name: "TruINJ".to_string(),
                symbol: "TRUINJ".to_string(),
                decimals: 18,
                total_supply: inj_to_mint.into(),
            }
        );
    }

    #[test]
    fn test_can_retrieve_marketing_info() {
        let (mut app, validator_addr) = mock_app_with_validator();
        let code_id = app.store_code(contract_wrapper());

        // instantiate the contract
        let owner = "owner".into_bech32();
        let msg = InstantiateMsg {
            treasury: "treasury".into_bech32(),
            default_validator: validator_addr,
        };
        let contract_addr = app
            .instantiate_contract(code_id, owner.clone(), &msg, &[], "staker-contract", None)
            .unwrap();

        // query the marketing Info
        let marketing_info: MarketingInfoResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::MarketingInfo {})
            .unwrap();

        assert_eq!(
            marketing_info,
            MarketingInfoResponse {
                project: Some("TruFin".to_string()),
                description: Some("TruFin's liquid staking token".to_string()),
                logo: Some(LogoInfo::Url(
                    "https://trufin-public-assets.s3.eu-west-2.amazonaws.com/truINJ-logo.svg"
                        .to_string()
                )),
                marketing: Some(owner),
            }
        );
    }
}
