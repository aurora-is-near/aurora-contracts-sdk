#[cfg(test)]
mod tests {
    use aurora_sdk_integration_tests::{
        aurora_engine::{self, AuroraEngine, ContractInput},
        aurora_engine_sdk::types,
        aurora_engine_types::{
            types::{Address, Wei},
            U256,
        },
        ethabi, tokio,
        utils::{self, ethabi::DeployedContract, forge},
        wnear,
        workspaces::{self, AccountId},
    };
    use std::path::Path;

    #[tokio::test]
    async fn test_contract() {
        let worker = workspaces::sandbox().await.unwrap();
        let engine = aurora_engine::deploy_latest(&worker).await.unwrap();
        let wnear = wnear::Wnear::deploy(&worker, &engine).await.unwrap();

        // Deploy Near contract
        tokio::fs::create_dir_all("../target/near/tic_tac_toe_on_near")
            .await
            .unwrap();
        let contract_bytes = utils::cargo::build_contract("../near-contract")
            .await
            .unwrap();
        let near_contract = worker.dev_deploy(&contract_bytes).await.unwrap();

        // Deploy solidity contract
        let solidity_contract = deploy_solidity_tic_tac_toe(
            &worker,
            &engine,
            near_contract.id(),
            wnear.aurora_token.address,
        )
        .await;

        // Approve proxy contract to spend owners's wNEAR
        engine
            .mint_wnear(
                &wnear,
                types::near_account_to_evm_address(engine.inner.id().as_bytes()),
                5_000_000_000_000_000_000_000_000,
            )
            .await
            .unwrap();
        let evm_input = wnear
            .aurora_token
            .create_approve_call_bytes(solidity_contract.inner.address, U256::MAX);
        let result = engine
            .call_evm_contract(wnear.aurora_token.address, evm_input, Wei::zero())
            .await
            .unwrap();
        aurora_engine::unwrap_success(result.status).unwrap();

        // Initialize the contract
        solidity_contract.init().await.unwrap();

        // Start a new game where the user plays as X.
        solidity_contract.new_game(true).await.unwrap();
        // User plays X in the center; computer responds with an O in the top left.
        let state = solidity_contract
            .take_turn(0x000000000100000000)
            .await
            .unwrap();
        assert_eq!(state, 0x110000000100000000);
        // User plays X in the top center; computer responds with blocking in the bottom center
        let state = solidity_contract
            .take_turn(0x000100000000000000)
            .await
            .unwrap();
        assert_eq!(state, 0x110100000100001100);
        // User plays X in the top right; computer responds with blocking in the bottom left
        let state = solidity_contract
            .take_turn(0x000001000000000000)
            .await
            .unwrap();
        assert_eq!(state, 0x110101000100111100);
        // User plays X in the middle right; computer responds by playing O in the middle left
        // This move ends the game with O getting 3 in a row.
        let state = solidity_contract
            .take_turn(0x000000000001000000)
            .await
            .unwrap();
        assert_eq!(state, 0x110101110101111100);
        // If user tries to take another turn they get a game over error
        let err = solidity_contract
            .take_turn(0x000000010000000000)
            .await
            .unwrap_err();
        let err_msg = format!("{err:?}");
        let expected_err_msg = format!("{:?}", b"Game Over")
            .replace('[', "")
            .replace(']', "");
        assert!(
            err_msg.contains(expected_err_msg.as_str()),
            "Unexpected error: {err_msg}"
        );
    }

    async fn deploy_solidity_tic_tac_toe<'a>(
        worker: &workspaces::Worker<workspaces::network::Sandbox>,
        engine: &'a AuroraEngine,
        tic_tac_toe_account_id: &AccountId,
        wnear_address: Address,
    ) -> TicTacToeContract<'a> {
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
            &["out", "TicTacToe.sol", "TicTacToe.json"],
        )
        .await
        .unwrap();
        let code = solidity_constructor.create_deploy_bytes_with_args(&[
            ethabi::Token::String(tic_tac_toe_account_id.as_str().into()),
            ethabi::Token::Address(wnear_address.raw()),
        ]);
        let contract_address = engine.deploy_evm_contract(code).await.unwrap();
        let inner = solidity_constructor.deployed_at(contract_address);

        // An account that represents a random Aurora user.
        // In reality this user does not need to have a NEAR account, they just need a way
        // to interact with Aurora. Typically this is a Secp256k1 private key they use to submit
        // signed transactions to the EVM.
        // For the sake of simplicity in this test we are choosing to not create a Secp256k1 key
        // and submit signed transactions, but you should imagine that is what is happening.
        let user = worker.dev_create_account().await.unwrap();

        TicTacToeContract {
            inner,
            engine,
            user,
        }
    }

    struct TicTacToeContract<'a> {
        inner: DeployedContract,
        engine: &'a AuroraEngine,
        user: workspaces::Account,
    }

    impl<'a> TicTacToeContract<'a> {
        async fn init(&self) -> anyhow::Result<()> {
            let result = self
                .engine
                .call_evm_contract(
                    self.inner.address,
                    ContractInput(
                        self.inner
                            .abi
                            .function("init")
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

        async fn new_game(&self, user_is_x: bool) -> anyhow::Result<()> {
            let player_preference = if user_is_x {
                ethabi::Token::Uint(1.into())
            } else {
                ethabi::Token::Uint(0.into())
            };
            let result = self
                .engine
                .call_evm_contract_with(
                    &self.user,
                    self.inner.address,
                    ContractInput(
                        self.inner
                            .abi
                            .function("newGame")
                            .unwrap()
                            .encode_input(&[player_preference])
                            .unwrap(),
                    ),
                    Wei::zero(),
                )
                .await?;
            aurora_engine::unwrap_success(result.status)?;
            Ok(())
        }

        // User makes a play; returns the board state after this move + the computer's response.
        async fn take_turn(&self, play: u128) -> anyhow::Result<u128> {
            let result = self
                .engine
                .call_evm_contract_with(
                    &self.user,
                    self.inner.address,
                    ContractInput(
                        self.inner
                            .abi
                            .function("takePlayerTurn")
                            .unwrap()
                            .encode_input(&[ethabi::Token::Uint(play.into())])
                            .unwrap(),
                    ),
                    Wei::zero(),
                )
                .await?;
            aurora_engine::unwrap_success(result.status)?;
            self.get_game_state().await
        }

        async fn get_game_state(&self) -> anyhow::Result<u128> {
            let user_address = types::near_account_to_evm_address(self.user.id().as_bytes());
            let result = self
                .engine
                .view_evm_contract(
                    self.inner.address,
                    ContractInput(
                        self.inner
                            .abi
                            .function("getGameState")
                            .unwrap()
                            .encode_input(&[ethabi::Token::Address(user_address.raw())])
                            .unwrap(),
                    ),
                    None,
                    Wei::zero(),
                )
                .await?;
            let output = aurora_engine::unwrap_success(result)?;
            let buf: [u8; 16] = output[16..32].try_into()?;
            // Using a bit mask to ignore the extra game over bytes and only keep the state bytes.
            Ok(u128::from_be_bytes(buf) & 0xffffffffffffffffff)
        }
    }
}
