pub mod helpers;

#[cfg(test)]
mod truinj {

    use crate::helpers::{
        self, instantiate_mock_cw20_receiver, instantiate_staker, mint_truinj, stake,
        wasm_execute_msg, whitelist_user,
    };
    use cosmwasm_std::{to_json_binary, Addr, Binary, Uint128, WasmMsg};
    use cw20::{LogoInfo, MarketingInfoResponse, TokenInfoResponse};
    use cw_multi_test::{Executor, IntoBech32};
    use helpers::{mint_inj, query_truinj_balance};
    use injective_staker::msg::{ExecuteMsg, QueryMsg};

    #[test]
    fn test_stake_mints_truinj() {
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

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
        // instantiate the contract
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        let anyone: Addr = "anyone".into_bech32();
        let recipient: Addr = "recipient".into_bech32();

        // mint INJ tokens to the 'anyone' user
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

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
        // instantiate the contract
        let owner = "owner".into_bech32();
        let (mut app, contract_addr, _) =
            instantiate_staker(owner.clone(), "treasury".into_bech32());

        // mint INJ tokens to the 'anyone' user
        let anyone: Addr = "anyone".into_bech32();
        let inj_to_mint = 10000000000000000000; // 10 INJ
        mint_inj(&mut app, &anyone, inj_to_mint);

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
        // instantiate the contract
        let owner = "owner".into_bech32();
        let (app, contract_addr, _) = instantiate_staker(owner.clone(), "treasury".into_bech32());

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

    #[test]
    fn test_can_send_truinj_to_a_contract() {
        let owner: Addr = "owner".into_bech32();
        let (mut app, staker_addr, _) = instantiate_staker(owner.clone(), "treasury".into_bech32());

        // instantiate a mock token receiver contract
        let receiver_contract_addr = instantiate_mock_cw20_receiver(&mut app, &owner);

        // mint TruINJ tokens to the sender account
        let sender = "sender".into_bech32();
        let token_amount = Uint128::from(1_000_000u128);
        mint_truinj(&mut app, &staker_addr, &owner, &sender, token_amount.u128());

        // send all TruINJ tokens to the receiver contract
        let response = app.execute(
            sender.clone(),
            wasm_execute_msg(
                &staker_addr,
                &ExecuteMsg::Send {
                    contract: receiver_contract_addr.to_string(),
                    amount: token_amount,
                    msg: Binary::default(),
                },
            )
            .into(),
        );
        assert!(response.is_ok());

        // verify the sender spent all their TruINJ tokens
        let sender_balance = query_truinj_balance(&app, &sender, &staker_addr);
        assert_eq!(sender_balance, 0u128);

        // verify the receiver contract received the expected amount of tokens
        let receiver_balance = query_truinj_balance(&app, &receiver_contract_addr, &staker_addr);
        assert_eq!(receiver_balance, token_amount.u128());
    }
}
