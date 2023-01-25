//! Convenience data-types and functions for deploying/interacting with the OpenZeppelin
//! ERC-20 contract: https://docs.openzeppelin.com/contracts/4.x/erc20#Presets

use crate::aurora_engine::ContractInput;
use aurora_engine_types::{types::Address, U256};

pub struct Constructor {
    pub code: Vec<u8>,
    pub abi: ethabi::Contract,
}

impl Constructor {
    pub async fn load() -> anyhow::Result<Self> {
        let code_hex = std::include_str!("../../res/ERC20PresetMinterPauser.bin");
        let code = hex::decode(code_hex)?;
        let abi_bytes = std::include_bytes!("../../res/ERC20PresetMinterPauser.abi");
        let abi = serde_json::from_slice(abi_bytes.as_slice())?;
        Ok(Self { code, abi })
    }

    pub fn deploy_code(&self, name: &str, symbol: &str) -> Vec<u8> {
        // Unwraps are safe because we statically know there is a constructor and it
        // takes two strings as input.
        self.abi
            .constructor()
            .unwrap()
            .encode_input(
                self.code.clone(),
                &[
                    ethabi::Token::String(name.to_string()),
                    ethabi::Token::String(symbol.to_string()),
                ],
            )
            .unwrap()
    }
}

pub struct ERC20 {
    pub abi: ethabi::Contract,
    pub address: Address,
}

impl ERC20 {
    pub fn mint(&self, recipient: Address, amount: U256) -> ContractInput {
        let data = self
            .abi
            .function("mint")
            .unwrap()
            .encode_input(&[
                ethabi::Token::Address(recipient.raw()),
                ethabi::Token::Uint(amount),
            ])
            .unwrap();
        ContractInput(data)
    }

    pub fn balance_of(&self, address: Address) -> ContractInput {
        let data = self
            .abi
            .function("balanceOf")
            .unwrap()
            .encode_input(&[ethabi::Token::Address(address.raw())])
            .unwrap();
        ContractInput(data)
    }

    pub fn approve(&self, spender: Address, amount: U256) -> ContractInput {
        let data = self
            .abi
            .function("approve")
            .unwrap()
            .encode_input(&[
                ethabi::Token::Address(spender.raw()),
                ethabi::Token::Uint(amount),
            ])
            .unwrap();
        ContractInput(data)
    }
}

pub trait ERC20DeployedAt {
    fn deployed_at(self, address: Address) -> ERC20;
}

impl ERC20DeployedAt for Constructor {
    fn deployed_at(self, address: Address) -> ERC20 {
        ERC20 {
            abi: self.abi,
            address,
        }
    }
}
