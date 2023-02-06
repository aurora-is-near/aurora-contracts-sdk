pub mod uniswap_utils;

#[cfg(test)]
mod tests {
    use crate::uniswap_utils::*;
    use aurora_sdk_integration_tests::{
        aurora_engine, aurora_engine_types::U256, tokio, utils, workspaces,
    };

    const MINT_AMOUNT: u64 = 1_000_000_000_000;
    const LIQUIDITY_AMOUNT: u64 = MINT_AMOUNT / 5;
    const OUTPUT_AMOUNT: u64 = LIQUIDITY_AMOUNT / 100;

    #[tokio::test]
    async fn test_contract() {
        let worker = workspaces::sandbox().await.unwrap();
        let engine = aurora_engine::deploy_latest(&worker).await.unwrap();

        // This is needed because of a quirk of how `cargo-near` works. It doesn't handle
        // cargo workspaces properly yet.
        tokio::fs::create_dir_all("../target/near/uniswap_from_near")
            .await
            .unwrap();
        let contract_bytes = utils::cargo::build_contract("../contract").await.unwrap();
        let contract = contract_interface::UniswapProxy {
            contract: worker.dev_deploy(&contract_bytes).await.unwrap(),
        };

        // Deploy uniswap contracts
        let ctx =
            UniswapTestContext::new(engine.clone(), contract.contract.as_account().clone()).await;

        // Deploy ERC-20 tokens to swap
        let token_a = ctx.create_token("A", "AAA", MINT_AMOUNT.into()).await;
        let token_b = ctx.create_token("B", "BBB", MINT_AMOUNT.into()).await;
        let _pool = ctx.create_pool(&token_a, &token_b).await;
        ctx.add_equal_liquidity(LIQUIDITY_AMOUNT.into(), &token_a, &token_b)
            .await;

        // Initialize our UniswapProxy contract
        contract
            .create(engine.inner.id(), &ctx.swap_router.0.address.encode())
            .await
            .unwrap();

        // The EVM address of our contract when it calls Aurora
        let contract_aurora_address =
            aurora_sdk_integration_tests::aurora_engine_sdk::types::near_account_to_evm_address(
                ctx.proxy_account.id().as_bytes(),
            );

        // Approve SwapRouter to spend our ERC-20 tokens so that we can do the swap
        ctx.approve_erc20(&token_a, ctx.swap_router.0.address, U256::MAX)
            .await;
        ctx.approve_erc20(&token_b, ctx.swap_router.0.address, U256::MAX)
            .await;

        // Use our contract to perform the swap
        contract
            .exact_output_single(token_a.address, token_b.address, OUTPUT_AMOUNT.into())
            .await
            .unwrap();

        // Confirm balances are correct
        let amount_a = engine
            .erc20_balance_of(&token_a, contract_aurora_address)
            .await
            .unwrap()
            .as_u64();
        let amount_b = engine
            .erc20_balance_of(&token_b, contract_aurora_address)
            .await
            .unwrap()
            .as_u64();

        // We lose a small amount of token_a to the pool fee
        let swap_fee = 1_976_829;
        assert_eq!(
            MINT_AMOUNT - LIQUIDITY_AMOUNT - OUTPUT_AMOUNT - swap_fee,
            amount_a
        );
        assert_eq!(MINT_AMOUNT - LIQUIDITY_AMOUNT + OUTPUT_AMOUNT, amount_b);
    }

    // This module contains convenience functions for interacting with the UniswapProxy contract
    // from the test code above.
    //
    // Future enhancement: should be able to derive this interface automatically using the
    // [near-abi](https://github.com/near/abi) project once it is mature enough.
    mod contract_interface {
        use aurora_sdk_integration_tests::{
            aurora_engine_types::{types::Address, U256},
            workspaces::{self, Contract},
        };
        use uniswap_from_near::SerializableExactOutputSingleParams;

        pub struct UniswapProxy {
            /// The `workspaces::Contract` instance here must have the UniswapProxy example
            /// contract deployed; it cannot be any `Contract`.
            pub contract: Contract,
        }

        impl UniswapProxy {
            pub async fn create(
                &self,
                aurora: &workspaces::AccountId,
                uniswap_address: &str,
            ) -> Result<(), workspaces::error::Error> {
                let result = self
                    .contract
                    .call("new")
                    .args_json(NewArgs {
                        aurora,
                        uniswap_address,
                    })
                    .max_gas()
                    .transact()
                    .await?;
                result.into_result()?;
                Ok(())
            }

            pub async fn exact_output_single(
                &self,
                token_in: Address,
                token_out: Address,
                amount_out: U256,
            ) -> Result<(), workspaces::error::Error> {
                let recipient = aurora_sdk_integration_tests::aurora_engine_sdk::types::near_account_to_evm_address(self.contract.id().as_bytes());
                let params = SerializableExactOutputSingleParams {
                    token_in: token_in.encode(),
                    token_out: token_out.encode(),
                    fee: None,
                    recipient: recipient.encode(),
                    deadline: None,
                    amount_out: amount_out.to_string(),
                    amount_in_max: None,
                    price_limit: None,
                };

                let result = self
                    .contract
                    .call("exact_output_single")
                    .args_json(ExactOutputSingleArgs { params })
                    .max_gas()
                    .transact()
                    .await?;

                // Note: the promise returned by `UniswapProxy::exact_output_single` is
                // fully resolved by the workspaces library. This `into_result` will
                // return an error if `UniswapProxy::exact_output_single` or
                // `UniswapProxy::parse_exact_output_single_result` fail.
                result.into_result()?;
                Ok(())
            }
        }

        #[derive(serde::Serialize)]
        struct NewArgs<'a> {
            aurora: &'a workspaces::AccountId,
            uniswap_address: &'a str,
        }

        #[derive(serde::Serialize)]
        struct ExactOutputSingleArgs {
            params: SerializableExactOutputSingleParams,
        }
    }
}
