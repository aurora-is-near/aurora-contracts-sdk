#[cfg(test)]
mod tests {
    use aurora_sdk_integration_tests::{
        aurora_engine::{self, AuroraEngine, ContractInput},
        aurora_engine_sdk::types::{self, near_account_to_evm_address},
        aurora_engine_types::{
            types::{Address, Wei},
            U256,
        },
        ethabi, tokio,
        utils::{self, ethabi::DeployedContract, forge},
        wnear,
        workspaces::{self, Account, AccountId},
    };
    use std::path::Path;

    #[tokio::test]
    async fn test_contract() {
        let worker = workspaces::sandbox().await.unwrap();
        let engine = aurora_engine::deploy_latest(&worker).await.unwrap();
        let wnear = wnear::Wnear::deploy(&worker, &engine).await.unwrap();

        // Deploy Near contract
        tokio::fs::create_dir_all("../target/near/ft_refund")
            .await
            .unwrap();
        tokio::fs::create_dir_all("../target/near/test_token")
            .await
            .unwrap();
        let contract_bytes = utils::cargo::build_contract("../near-contract")
            .await
            .unwrap();
        let near_contract = worker.dev_deploy(&contract_bytes).await.unwrap();
        let contract_bytes = utils::cargo::build_contract("../test-token").await.unwrap();
        let test_token_contract = worker.dev_deploy(&contract_bytes).await.unwrap();

        let account = worker.dev_create_account().await.unwrap();
        let address = near_account_to_evm_address(account.id().as_bytes());

        let test_erc20 = engine
            .bridge_nep141(test_token_contract.id())
            .await
            .unwrap();

        // Deploy solidity contract
        let solidity_contract =
            deploy_solidity_contract(&engine, near_contract.id(), wnear.aurora_token.address).await;

        let xcc_account = format!(
            "{}.{}",
            solidity_contract.inner.address.encode(),
            engine.inner.id()
        );

        let xcc_implicit_address = types::near_account_to_evm_address(xcc_account.as_bytes());

        // Mint WNEAR for the engine implicit account to use
        engine
            .mint_wnear(
                &wnear,
                types::near_account_to_evm_address(engine.inner.id().as_bytes()),
                5_000_000_000_000_000_000_000_000,
            )
            .await
            .unwrap();

        // Mint WNEAR for the proxy contract to use
        engine
            .mint_wnear(
                &wnear,
                solidity_contract.inner.address,
                5_000_000_000_000_000_000_000_000,
            )
            .await
            .unwrap();

        // Mint WNEAR for the account to use
        engine
            .mint_wnear(&wnear, address, 5_000_000_000_000_000_000_000_000)
            .await
            .unwrap();

        // Mint WNEAR for the XCC implicit account to use
        engine
            .mint_wnear(
                &wnear,
                xcc_implicit_address,
                5_000_000_000_000_000_000_000_000,
            )
            .await
            .unwrap();

        // Approve proxy contract to spend user's WNEAR
        let result = engine
            .call_evm_contract_with(
                &account,
                wnear.aurora_token.address,
                wnear
                    .aurora_token
                    .create_approve_call_bytes(solidity_contract.inner.address, U256::MAX),
                Wei::zero(),
            )
            .await
            .unwrap();
        aurora_engine::unwrap_success(result.status).unwrap();

        // Initialize test token contract
        test_token_contract
            .call("new")
            .args_json(("Token", "TKN", 18))
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();
        // Pay storage deposit for Aurora contract
        test_token_contract
            .call("storage_deposit")
            .args_json((engine.inner.id(), true))
            .deposit(10_000_000_000_000_000_000_000)
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();
        // Pay storage deposit for Near contract
        test_token_contract
            .call("storage_deposit")
            .args_json((near_contract.id(), true))
            .deposit(10_000_000_000_000_000_000_000)
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();
        // Pay storage deposit for XCC account
        test_token_contract
            .call("storage_deposit")
            .args_json((&xcc_account, true))
            .deposit(10_000_000_000_000_000_000_000)
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();
        // Pay storage deposit for sender account
        test_token_contract
            .call("storage_deposit")
            .args_json((account.id(), true))
            .deposit(10_000_000_000_000_000_000_000)
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();
        // Mint test tokens
        test_token_contract
            .call("mint")
            .args_json((account.id(), "1"))
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();
        // approve ERC20 for solidity contract
        let result = engine
            .call_evm_contract_with(
                &account,
                test_erc20.address,
                test_erc20.create_approve_call_bytes(solidity_contract.inner.address, "1".into()),
                Wei::zero(),
            )
            .await
            .unwrap();
        aurora_engine::unwrap_success(result.status).unwrap();
        // Bridge test token
        account
            .call(test_token_contract.id(), "ft_transfer_call")
            .args_json((engine.inner.id(), "1", "null", address.encode()))
            .max_gas()
            .deposit(1)
            .transact()
            .await
            .unwrap()
            .into_result()
            .unwrap();

        // assert ERC20 amount after bridging
        let erc20_balance = engine.erc20_balance_of(&test_erc20, address).await.unwrap();
        assert_eq!(erc20_balance, "1".into());

        // Approve the proxy contract to spend the WNEAR of the xcc implicit address
        solidity_contract.approve_wnear(&account).await.unwrap();

        // View call to check the above transaction had the intended effect
        let approve_amount = {
            let result = engine
                .view_evm_contract(
                    wnear.aurora_token.address,
                    wnear.aurora_token.create_allowance_call_bytes(
                        xcc_implicit_address,
                        solidity_contract.inner.address,
                    ),
                    None,
                    Wei::zero(),
                )
                .await
                .unwrap();
            aurora_engine::unwrap_success(result)
                .map(|bytes| U256::from_big_endian(&bytes))
                .unwrap()
        };
        assert_eq!(approve_amount, U256::MAX);

        // Call the solidity contract. It will
        // - bridge the tokens from Aurora to Near
        // - call `ft_transfer_call` and send FT to our Near contract
        // - Near contract will tell token contract to do the refund
        // - refund will be sent to XCC account
        // - XCC account will do refund to EVM address via bridging back the token
        solidity_contract
            .ft_transfer_call(
                &account,
                &test_erc20.address,
                test_token_contract.id().to_string(),
                1,
            )
            .await
            .unwrap();

        // assert ERC20 amount after calling solidity contract
        let erc20_balance = engine.erc20_balance_of(&test_erc20, address).await.unwrap();
        assert_eq!(erc20_balance, "1".into());
    }

    async fn deploy_solidity_contract<'a>(
        engine: &'a AuroraEngine,
        contract_id: &AccountId,
        wnear_address: Address,
    ) -> SolidityContract<'a> {
        let contract_path = "../solidity-contract";
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
        let solidity_constructor = forge::forge_build(
            contract_path,
            &[format!(
                "aurora-sdk/AuroraSdk.sol:AuroraSdk:0x{}",
                aurora_sdk_lib.encode()
            )],
            &["out", "FtRefund.sol", "FtRefund.json"],
        )
        .await
        .unwrap();
        let code = solidity_constructor.create_deploy_bytes_with_args(&[
            ethabi::Token::String(contract_id.as_str().into()),
            ethabi::Token::Address(wnear_address.raw()),
        ]);
        let contract_address = engine.deploy_evm_contract(code).await.unwrap();
        let inner = solidity_constructor.deployed_at(contract_address);

        SolidityContract { inner, engine }
    }

    struct SolidityContract<'a> {
        inner: DeployedContract,
        engine: &'a AuroraEngine,
    }

    impl<'a> SolidityContract<'a> {
        async fn ft_transfer_call<T: Into<ethabi::Uint>>(
            &self,
            account: &Account,
            token_address: &Address,
            token_id: String,
            amount: T,
        ) -> anyhow::Result<()> {
            let result = self
                .engine
                .call_evm_contract_with(
                    account,
                    self.inner.address,
                    ContractInput(
                        self.inner
                            .abi
                            .function("ftTransferCall")
                            .unwrap()
                            .encode_input(&[
                                ethabi::Token::Address(token_address.raw()),
                                ethabi::Token::String(token_id),
                                ethabi::Token::Uint(amount.into()),
                            ])
                            .unwrap(),
                    ),
                    Wei::zero(),
                )
                .await?;
            aurora_engine::unwrap_success(result.status)?;
            Ok(())
        }

        async fn approve_wnear(&self, account: &Account) -> anyhow::Result<()> {
            let result = self
                .engine
                .verbose_call_evm_contract_with(
                    account,
                    self.inner.address,
                    ContractInput(
                        self.inner
                            .abi
                            .function("approveWNEAR")
                            .unwrap()
                            .encode_input(&[])
                            .unwrap(),
                    ),
                    Wei::zero(),
                )
                .await?;
            aurora_engine::unwrap_success(result.status)?;
            Ok(())
        }
    }
}
