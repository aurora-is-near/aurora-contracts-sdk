#[cfg(test)]
mod tests {
    use aurora_sdk_integration_tests::{
        aurora_engine::{self, erc20, unwrap_success, AuroraEngine, ContractInput},
        aurora_engine_sdk::types::near_account_to_evm_address,
        aurora_engine_types::{
            types::{Address, Wei},
            U256,
        },
        ethabi, tokio,
        utils::{ethabi::DeployedContract, forge},
        workspaces::{self, Account},
    };

    const ORACLE_FEE: u64 = 100;
    const BOUNTY_PR_URL: &str = "https://github.com/some-org/some-repo/pull/666";
    const BOUNTY_REWARD: u64 = 500;
    const BOUNTY_DURATION: u64 = 5_000_000;

    #[tokio::test]
    async fn test_contract() {
        let worker = workspaces::sandbox().await.unwrap();
        let engine = aurora_engine::deploy_latest(&worker).await.unwrap();

        // There are multiple roles involved with this system:
        // 1. the oracle owner (can do admin actions on the oracle contract)
        // 2. the oracle backend (submits responses for the oracle contract)
        // 3. the bounty creator (sets the bounty)
        // 4. the developer (completes the bounty)
        // Additionally there is the fee token the oracle contract changes for requests and
        // a token that bounty reward is denominated in (you can imagine this as USDT for example).

        let oracle_owner = worker.dev_create_account().await.unwrap();
        let oracle_backend = worker.dev_create_account().await.unwrap();
        let bounty_creator = worker.dev_create_account().await.unwrap();
        let bounty_dev = worker.dev_create_account().await.unwrap();
        let oracle_token = deploy_erc20(&engine, "OracleCoin", "ORC").await;
        let reward_token = deploy_erc20(&engine, "BountyCoin", "BYC").await;

        // The bounty creator has some reward tokens to give away in the bounty.
        let bounty_creator_address = near_account_to_evm_address(bounty_creator.id().as_bytes());
        let calldata = oracle_token
            .create_mint_call_bytes(bounty_creator_address, (10 * BOUNTY_REWARD).into());
        let result = engine
            .call_evm_contract(reward_token.address, calldata, Wei::zero())
            .await
            .unwrap();
        unwrap_success(result.status).unwrap();

        // Deploy Oracle contract.
        let oracle_backend_address = near_account_to_evm_address(oracle_backend.id().as_bytes());
        let oracle_contract = deploy_oracle(
            &engine,
            &oracle_owner,
            oracle_backend_address,
            &oracle_token,
        )
        .await;

        // Deploy Bounty contract.
        let bounty_contract = deploy_bounty_contract(&engine, &oracle_contract).await;

        // We must give the Bounty contract some Oracle tokens so that it can use the oracle.
        let calldata =
            oracle_token.create_mint_call_bytes(bounty_contract.address, (10 * ORACLE_FEE).into());
        let result = engine
            .call_evm_contract(oracle_token.address, calldata, Wei::zero())
            .await
            .unwrap();
        unwrap_success(result.status).unwrap();

        // The bounty creator must approve the bounty contract to take the reward tokens
        let calldata = reward_token.create_approve_call_bytes(bounty_contract.address, U256::MAX);
        let result = engine
            .call_evm_contract_with(&bounty_creator, reward_token.address, calldata, Wei::zero())
            .await
            .unwrap();
        unwrap_success(result.status).unwrap();

        // Create a new bounty.
        let calldata = bounty_contract
            .abi
            .function("createBounty")
            .unwrap()
            .encode_input(&[
                ethabi::Token::String(BOUNTY_PR_URL.into()),
                ethabi::Token::Address(reward_token.address.raw()),
                ethabi::Token::Uint(BOUNTY_REWARD.into()),
                ethabi::Token::Uint(BOUNTY_DURATION.into()),
            ])
            .unwrap();
        let result = engine
            .call_evm_contract_with(
                &bounty_creator,
                bounty_contract.address,
                ContractInput(calldata),
                Wei::zero(),
            )
            .await
            .unwrap();
        let bounty_id = U256::from_big_endian(&unwrap_success(result.status).unwrap());

        // TODO: list bounties and confirm it is present.

        // Developer submits intent to complete bounty.
        let bounty_dev_address = near_account_to_evm_address(bounty_dev.id().as_bytes());
        let calldata = bounty_contract
            .abi
            .function("submitIntent")
            .unwrap()
            .encode_input(&[
                ethabi::Token::Uint(bounty_id),
                ethabi::Token::Address(bounty_dev_address.raw()),
                ethabi::Token::String("dev_username".into()),
            ])
            .unwrap();
        let result = engine
            .call_evm_contract_with(
                &bounty_dev,
                bounty_contract.address,
                ContractInput(calldata),
                Wei::zero(),
            )
            .await
            .unwrap();
        unwrap_success(result.status).unwrap();

        // Some time later the work is finished.
        // Developer begins the process of claiming the bounty
        let calldata = bounty_contract
            .abi
            .function("tryClaimBounty")
            .unwrap()
            .encode_input(&[ethabi::Token::Uint(bounty_id)])
            .unwrap();
        let result = engine
            .call_evm_contract_with(
                &bounty_dev,
                bounty_contract.address,
                ContractInput(calldata),
                Wei::zero(),
            )
            .await
            .unwrap();
        let result = ethabi::decode(
            &[ethabi::ParamType::Bool],
            &unwrap_success(result.status).unwrap(),
        )
        .unwrap();
        assert_eq!(result, vec![ethabi::Token::Bool(true)]);

        // The above transactions creates an event which the oracle backend detects.
        // It queries the GH API and then returns a response.
        // For the sake of this example we'll assume the result is `true`.
        let calldata = oracle_contract
            .abi
            .function("respond")
            .unwrap()
            .encode_input(&[
                // TODO: hard-coding the query ID here, but could parse it from the above event instead
                ethabi::Token::Uint(1.into()),
                ethabi::Token::Bool(true),
            ])
            .unwrap();
        let result = engine
            .call_evm_contract_with(
                &oracle_backend,
                oracle_contract.address,
                ContractInput(calldata),
                Wei::zero(),
            )
            .await
            .unwrap();
        unwrap_success(result.status).unwrap();

        // The developer can now finish claiming their bounty reward.
        let calldata = bounty_contract
            .abi
            .function("finishClaimBounty")
            .unwrap()
            .encode_input(&[ethabi::Token::Uint(bounty_id)])
            .unwrap();
        let result = engine
            .call_evm_contract_with(
                &bounty_dev,
                bounty_contract.address,
                ContractInput(calldata),
                Wei::zero(),
            )
            .await
            .unwrap();
        let result = ethabi::decode(
            &[ethabi::ParamType::Bool],
            &unwrap_success(result.status).unwrap(),
        )
        .unwrap();
        assert_eq!(result, vec![ethabi::Token::Bool(true)]);

        // Confirm the developer received their reward
        let calldata = reward_token.create_balance_of_call_bytes(bounty_dev_address);
        let result = engine
            .view_evm_contract(reward_token.address, calldata, None, Wei::zero())
            .await
            .unwrap();
        let amount = U256::from_big_endian(&unwrap_success(result).unwrap());
        assert_eq!(amount, U256::from(BOUNTY_REWARD));
    }

    async fn deploy_oracle(
        engine: &AuroraEngine,
        oracle_owner: &Account,
        oracle_backend: Address,
        oracle_token: &erc20::ERC20,
    ) -> DeployedContract {
        let contract_path = "../solidity";
        let constructor =
            forge::forge_build(contract_path, &[], &["out", "Oracle.sol", "GHOracle.json"])
                .await
                .unwrap();
        let deploy_bytes = constructor.create_deploy_bytes_with_args(&[
            ethabi::Token::Address(oracle_backend.raw()),
            ethabi::Token::Address(oracle_token.address.raw()),
            ethabi::Token::Uint(ORACLE_FEE.into()),
        ]);
        let address = engine
            .deploy_evm_contract_with(oracle_owner, deploy_bytes)
            .await
            .unwrap();
        constructor.deployed_at(address)
    }

    async fn deploy_bounty_contract(
        engine: &AuroraEngine,
        oracle_contract: &DeployedContract,
    ) -> DeployedContract {
        let contract_path = "../solidity";
        let constructor = forge::forge_build(
            contract_path,
            &[],
            &["out", "Bounty.sol", "BountyProgram.json"],
        )
        .await
        .unwrap();
        let deploy_bytes = constructor.create_deploy_bytes_with_args(&[ethabi::Token::Address(
            oracle_contract.address.raw(),
        )]);
        let address = engine.deploy_evm_contract(deploy_bytes).await.unwrap();
        constructor.deployed_at(address)
    }

    async fn deploy_erc20(engine: &AuroraEngine, name: &str, symbol: &str) -> erc20::ERC20 {
        use erc20::ERC20DeployedAt;

        let constructor = erc20::Constructor::load().await.unwrap();
        let deploy_bytes = constructor.create_deploy_bytes(name, symbol);
        let address = engine.deploy_evm_contract(deploy_bytes).await.unwrap();
        constructor.deployed_at(address)
    }
}
