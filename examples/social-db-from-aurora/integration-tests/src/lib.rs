#[cfg(test)]
mod tests {
    use aurora_sdk_integration_tests::{
        aurora_engine::{self, AuroraEngine},
        aurora_engine_types::{
            parameters::engine::{CallArgs, FunctionCallArgsV1},
            types::{Address, Wei},
            U256,
        },
        ethabi, tokio,
        utils::{ethabi::DeployedContract, forge, process},
        wnear,
        workspaces::{self, AccountId},
    };
    use std::path::Path;

    const ATTACHED_NEAR: u128 = 5_000_000_000_000_000_000_000_000;
    const NEAR_DEPOSIT: u128 = 2_000_000_000_000_000_000_000_000;

    // See https://github.com/NearSocial/standards/blob/main/types/common/Metadata.md for schema
    #[derive(serde::Serialize)]
    struct Profile {
        name: String,
        #[serde(default)]
        description: Option<String>,
        tags: serde_json::Map<String, serde_json::Value>,
    }

    // See https://github.com/NearSocial/standards/blob/main/types/Root.md
    // (Most fields not included for the sake of brevity in the example)
    #[derive(serde::Serialize)]
    struct Root {
        #[serde(default)]
        profile: Option<Profile>,
    }

    // See https://github.com/NearSocial/social-db/blob/39016e654739b0a3e8cb7ffaea4b03157c4aea6e/contract/src/api.rs#L135
    #[derive(serde::Serialize)]
    struct SetArg {
        data: serde_json::Value,
    }

    #[tokio::test]
    async fn test_contract() {
        let worker = workspaces::sandbox().await.unwrap();
        let engine = aurora_engine::deploy_latest(&worker).await.unwrap();
        let wnear = wnear::Wnear::deploy(&worker, &engine).await.unwrap();

        // Deploy and initialize Social DB contract
        let social_db = deploy_social_db(&worker).await;
        social_db
            .call("new")
            .args(Vec::new())
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();
        // The contract must be made "Live" to accept transactions.
        social_db
            .call("set_status")
            .args_json(serde_json::json!({"status": "Live"}))
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();

        // An account that is a proxy for the Aurora user.
        // In reality this user does not need to have a NEAR account, they just need a way
        // to interact with Aurora. Typically this is a Secp256k1 private key they use to submit
        // signed transactions to the EVM.
        // For the sake of simplicity in this test we are choosing to not create a Secp256k1 key
        // and submit signed transactions, but you should imagine that is what is happening.
        let user_account = worker.dev_create_account().await.unwrap();
        let user_address =
            aurora_sdk_integration_tests::aurora_engine_sdk::types::near_account_to_evm_address(
                user_account.id().as_bytes(),
            );

        // Deploy EVM contract (`SocialDB.sol`) that will be the proxy on Aurora for the Social DB contract on Near
        let contract = deploy_social_db_sol_contract(
            &engine,
            &user_account,
            social_db.id(),
            wnear.aurora_token.address,
        )
        .await;

        // Give user some wNEAR to use for XCC
        engine
            .mint_wnear(&wnear, user_address, 2 * (ATTACHED_NEAR + NEAR_DEPOSIT))
            .await
            .unwrap();

        // Approve proxy contract to spend user's wNEAR
        let evm_input = wnear
            .aurora_token
            .create_approve_call_bytes(contract.address, U256::MAX);
        let result = engine
            .call_evm_contract_with(
                &user_account,
                wnear.aurora_token.address,
                evm_input,
                Wei::zero(),
            )
            .await
            .unwrap();
        aurora_engine::unwrap_success(result.status).unwrap();

        // Call contract to set profile data in Social DB
        let set_data = Root {
            profile: Some(Profile {
                name: format!("{}.aurora", contract.address.encode()),
                description: Some("An EVM contrct proxy to Social DB on Near".into()),
                tags: make_tags(&["Aurora", "contract", "EVM", "proxy"]),
            }),
        };
        let account_key = format!("{}.{}", contract.address.encode(), engine.inner.id());
        let set_data = serde_json::json!({
            account_key.clone(): set_data,
        });
        let contract_args = contract.create_call_method_bytes_with_args(
            "set",
            &[
                ethabi::Token::Uint(ATTACHED_NEAR.into()),
                ethabi::Token::Bytes(
                    serde_json::to_vec(&SetArg {
                        data: set_data.clone(),
                    })
                    .unwrap(),
                ),
            ],
        );

        let call_args = CallArgs::V1(FunctionCallArgsV1 {
            contract: contract.address,
            input: contract_args,
        });
        let outcome = user_account
            .call(engine.inner.id(), "call")
            .args_borsh(call_args)
            .max_gas()
            .transact()
            .await
            .unwrap();

        assert!(
            outcome.failures().is_empty(),
            "Call to set failed: {:?}",
            outcome.failures()
        );

        // Confirm the data was written to Social DB
        let profile_key = format!("{account_key}/profile/**");
        let result = social_db
            .call("get")
            .args_json(serde_json::json!({
                "keys": [profile_key],
                "options": serde_json::Value::Null,
            }))
            .transact()
            .await
            .unwrap();
        let value: serde_json::Value = result.json().unwrap();
        assert_eq!(value, set_data);
    }

    async fn deploy_social_db_sol_contract(
        engine: &AuroraEngine,
        user_account: &workspaces::Account,
        social_db_account_id: &AccountId,
        wnear_address: Address,
    ) -> DeployedContract {
        let contract_path = "../contract";
        let aurora_sdk_path = Path::new(contract_path).join("lib/aurora-solidity-sdk");
        let codec_lib = forge::deploy_codec_lib(&aurora_sdk_path, engine)
            .await
            .unwrap();
        let utils_lib = forge::deploy_utils_lib(&aurora_sdk_path, engine)
            .await
            .unwrap();
        let aurora_sdk_lib =
            forge::deploy_aurora_sdk_lib(&aurora_sdk_path, engine, codec_lib, utils_lib)
                .await
                .unwrap();
        let constructor = forge::forge_build(
            contract_path,
            &[format!(
                "aurora-sdk/AuroraSdk.sol:AuroraSdk:0x{}",
                aurora_sdk_lib.encode()
            )],
            &["out", "SocialDB.sol", "SocialDB.json"],
        )
        .await
        .unwrap();
        let deploy_bytes = constructor.create_deploy_bytes_with_args(&[
            ethabi::Token::String(social_db_account_id.to_string()),
            ethabi::Token::Address(wnear_address.raw()),
        ]);
        let address = engine
            .deploy_evm_contract_with(user_account, deploy_bytes)
            .await
            .unwrap();
        constructor.deployed_at(address)
    }

    async fn deploy_social_db(
        worker: &workspaces::Worker<workspaces::network::Sandbox>,
    ) -> workspaces::Contract {
        let contract_path = Path::new("../social-db");
        let output = tokio::process::Command::new("cargo")
            .current_dir(contract_path)
            .env("RUSTFLAGS", "-C link-arg=-s")
            .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
            .output()
            .await
            .unwrap();
        process::require_success(&output).unwrap();
        let artifact_path =
            contract_path.join("target/wasm32-unknown-unknown/release/contract.wasm");
        let wasm_bytes = tokio::fs::read(artifact_path).await.unwrap();
        worker.dev_deploy(&wasm_bytes).await.unwrap()
    }

    fn make_tags(tags: &[&str]) -> serde_json::Map<String, serde_json::Value> {
        tags.iter()
            .map(|t| (t.to_string(), serde_json::Value::String(String::new())))
            .collect()
    }
}
