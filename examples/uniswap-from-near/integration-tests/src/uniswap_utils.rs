use aurora_sdk_integration_tests::{
    aurora_engine::{self, erc20, AuroraEngine, ContractInput},
    aurora_engine_types::{
        types::{Address, Wei},
        U256,
    },
    ethabi,
    utils::ethabi::{ContractConstructor, DeployedContract},
    workspaces,
};
use std::ops::Not;
use std::path::Path;

const RES_PATH: &str = "res";
const POOL_FEE: u64 = 500;

pub struct FactoryConstructor(pub ContractConstructor);

pub struct Factory(pub DeployedContract);

pub struct Pool(pub DeployedContract);

pub struct PositionManagerConstructor(pub ContractConstructor);

pub struct PositionManager(pub DeployedContract);

pub struct SwapRouterConstructor(pub ContractConstructor);

pub struct SwapRouter(pub DeployedContract);

pub struct MintParams {
    pub token0: Address,
    pub token1: Address,
    pub fee: u64,
    pub tick_lower: i64,
    pub tick_upper: i64,
    pub amount0_desired: U256,
    pub amount1_desired: U256,
    pub amount0_min: U256,
    pub amount1_min: U256,
    pub recipient: Address,
    pub deadline: U256,
}

impl FactoryConstructor {
    pub fn load() -> Self {
        // Note the various json files in `RES_PATH` (e.g. UniswapV3Factory.json) are obtained
        // by compiling the Uniswap Solidity contracts (using Hardhat).
        let contract_path = Path::new(RES_PATH).join("UniswapV3Factory.json");
        Self(ContractConstructor::from_extended_json(contract_path))
    }

    pub fn deploy(&self) -> Vec<u8> {
        self.0.deploy_without_args()
    }

    pub fn deployed_at(self, address: Address) -> Factory {
        Factory(self.0.deployed_at(address))
    }
}

impl PositionManagerConstructor {
    pub fn load() -> Self {
        let contract_path = Path::new(RES_PATH).join("NonfungiblePositionManager.json");
        Self(ContractConstructor::from_extended_json(contract_path))
    }

    pub fn deploy(
        &self,
        factory: Address,
        wrapped_eth: Address,
        token_descriptor: Address,
    ) -> Vec<u8> {
        self.0.deploy_with_args(&[
            ethabi::Token::Address(factory.raw()),
            ethabi::Token::Address(wrapped_eth.raw()),
            ethabi::Token::Address(token_descriptor.raw()),
        ])
    }

    pub fn deployed_at(self, address: Address) -> PositionManager {
        PositionManager(self.0.deployed_at(address))
    }
}

impl Factory {
    pub fn create_pool(&self, token_a: Address, token_b: Address, fee: U256) -> ContractInput {
        let data = self.0.call_method_with_args(
            "createPool",
            &[
                ethabi::Token::Address(token_a.raw()),
                ethabi::Token::Address(token_b.raw()),
                ethabi::Token::Uint(fee),
            ],
        );
        ContractInput(data)
    }
}

impl Pool {
    pub fn from_address(address: Address) -> Self {
        let contract_path = Path::new(RES_PATH).join("UniswapV3Pool.json");
        let constructor = ContractConstructor::from_extended_json(contract_path);

        Self(DeployedContract {
            abi: constructor.abi,
            address,
        })
    }

    pub fn initialize(&self, price: U256) -> ContractInput {
        let data = self
            .0
            .call_method_with_args("initialize", &[ethabi::Token::Uint(price)]);
        ContractInput(data)
    }
}

impl PositionManager {
    pub fn mint(&self, params: MintParams) -> ContractInput {
        let tick_lower = Self::i64_to_u256(params.tick_lower);
        let tick_upper = Self::i64_to_u256(params.tick_upper);
        let data = self.0.call_method_with_args(
            "mint",
            &[ethabi::Token::Tuple(vec![
                ethabi::Token::Address(params.token0.raw()),
                ethabi::Token::Address(params.token1.raw()),
                ethabi::Token::Uint(params.fee.into()),
                ethabi::Token::Int(tick_lower),
                ethabi::Token::Int(tick_upper),
                ethabi::Token::Uint(params.amount0_desired),
                ethabi::Token::Uint(params.amount1_desired),
                ethabi::Token::Uint(params.amount0_min),
                ethabi::Token::Uint(params.amount1_min),
                ethabi::Token::Address(params.recipient.raw()),
                ethabi::Token::Uint(params.deadline),
            ])],
        );
        ContractInput(data)
    }

    fn i64_to_u256(x: i64) -> U256 {
        let y = U256::from(x.abs());
        if x < 0 {
            // compute two's complement to get negative number
            y.not().overflowing_add(U256::one()).0
        } else {
            y
        }
    }
}

impl SwapRouterConstructor {
    pub fn load() -> Self {
        let contract_path = Path::new(RES_PATH).join("SwapRouter.json");
        Self(ContractConstructor::from_extended_json(contract_path))
    }

    pub fn deploy(&self, factory: Address, wrapped_eth: Address) -> Vec<u8> {
        self.0.deploy_with_args(&[
            ethabi::Token::Address(factory.raw()),
            ethabi::Token::Address(wrapped_eth.raw()),
        ])
    }

    pub fn deployed_at(self, address: Address) -> SwapRouter {
        SwapRouter(self.0.deployed_at(address))
    }
}

pub struct UniswapTestContext {
    pub factory: Factory,
    pub manager: PositionManager,
    pub swap_router: SwapRouter,
    pub aurora: AuroraEngine,
    pub proxy_account: workspaces::Account,
}

impl UniswapTestContext {
    pub async fn new(aurora: AuroraEngine, proxy_account: workspaces::Account) -> Self {
        let factory_constructor = FactoryConstructor::load();
        let factory_address = aurora
            .deploy_evm_contract(factory_constructor.deploy())
            .await
            .unwrap();
        // Just deploying any ERC-20 contract; doesn't need to actually be wrapped ETH because
        // it's not used for wrapping or unwrapping in this test.
        let weth_address = {
            let constructor = erc20::Constructor::load().await.unwrap();
            aurora
                .deploy_evm_contract(constructor.deploy_code("Wrapped Ether", "WETH"))
                .await
                .unwrap()
        };
        let manager_constructor = PositionManagerConstructor::load();
        let manager_address = aurora
            .deploy_evm_contract(manager_constructor.deploy(
                factory_address,
                weth_address,
                Address::from_array([0; 20]),
            ))
            .await
            .unwrap();
        let router_constructor = SwapRouterConstructor::load();
        let router_address = aurora
            .deploy_evm_contract(router_constructor.deploy(factory_address, weth_address))
            .await
            .unwrap();

        Self {
            factory: factory_constructor.deployed_at(factory_address),
            manager: manager_constructor.deployed_at(manager_address),
            swap_router: router_constructor.deployed_at(router_address),
            aurora,
            proxy_account,
        }
    }

    pub async fn create_token(&self, name: &str, symbol: &str, mint_amount: U256) -> erc20::ERC20 {
        let constructor = erc20::Constructor::load().await.unwrap();
        let address = self
            .aurora
            .deploy_evm_contract(constructor.deploy_code(name, symbol))
            .await
            .unwrap();
        let token = erc20::ERC20 {
            abi: constructor.abi,
            address,
        };
        let recipient =
            aurora_sdk_integration_tests::aurora_engine_sdk::types::near_account_to_evm_address(
                self.proxy_account.id().as_bytes(),
            );
        let mint_result = self
            .aurora
            .call_evm_contract(address, token.mint(recipient, mint_amount), Wei::zero())
            .await
            .unwrap();
        assert!(mint_result.status.is_ok(), "Mint failed: {:?}", mint_result);
        token
    }

    pub async fn create_pool(&self, token_a: &erc20::ERC20, token_b: &erc20::ERC20) -> Pool {
        let input = self
            .factory
            .create_pool(token_a.address, token_b.address, POOL_FEE.into());
        let result = self
            .aurora
            .call_evm_contract(self.factory.0.address, input, Wei::zero())
            .await
            .unwrap();
        assert!(result.status.is_ok(), "Failed to create pool: {:?}", result);

        let output = aurora_engine::unwrap_success(result.status).unwrap();
        let address = Address::try_from_slice(&output[12..]).unwrap();
        let pool = Pool::from_address(address);

        // 2^96 corresponds to a price ratio of 1
        let input = pool.initialize(U256::from(2).pow(U256::from(96)));
        let result = self
            .aurora
            .call_evm_contract(address, input, Wei::zero())
            .await
            .unwrap();
        assert!(
            result.status.is_ok(),
            "Failed to initialize pool: {:?}",
            result
        );

        pool
    }

    pub async fn add_equal_liquidity(
        &self,
        amount: U256,
        token_a: &erc20::ERC20,
        token_b: &erc20::ERC20,
    ) {
        self.approve_erc20(token_a, self.manager.0.address, U256::MAX)
            .await;
        self.approve_erc20(token_b, self.manager.0.address, U256::MAX)
            .await;

        let token0 = std::cmp::min(token_a.address, token_b.address);
        let token1 = std::cmp::max(token_a.address, token_b.address);
        let recipient =
            aurora_sdk_integration_tests::aurora_engine_sdk::types::near_account_to_evm_address(
                self.proxy_account.id().as_bytes(),
            );
        let params = MintParams {
            token0,
            token1,
            fee: POOL_FEE,
            tick_lower: -1000,
            tick_upper: 1000,
            amount0_desired: amount,
            amount1_desired: amount,
            amount0_min: U256::one(),
            amount1_min: U256::one(),
            recipient,
            deadline: U256::MAX, // no deadline
        };
        let input = self.manager.mint(params);
        let result = self
            .aurora
            .call_evm_contract_with(
                &self.proxy_account,
                self.manager.0.address,
                input,
                Wei::zero(),
            )
            .await
            .unwrap();
        assert!(
            result.status.is_ok(),
            "Failed to mint liquidity: {:?}",
            result
        );
    }

    pub async fn approve_erc20(&self, token: &erc20::ERC20, spender: Address, amount: U256) {
        let input = token.approve(spender, amount);
        let result = self
            .aurora
            .call_evm_contract_with(&self.proxy_account, token.address, input, Wei::zero())
            .await
            .unwrap();
        assert!(
            result.status.is_ok(),
            "Failed to approve ERC-20 token: {:?}",
            result
        );
    }
}
