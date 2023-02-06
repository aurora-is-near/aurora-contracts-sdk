pub use aurora_engine_sdk::{
    ecrecover, keccak, sha256, types::near_account_to_evm_address, ECRecoverErr,
};
pub use aurora_engine_types::{
    self,
    account_id::AccountId,
    parameters::{
        self,
        engine::{
            CallArgs, FunctionCallArgsV1, FunctionCallArgsV2, GetStorageAtArgs, SubmitResult,
            TransactionStatus, ViewCallArgs,
        },
    },
    types::{self, address::error::AddressError, Address, Wei},
    H160, H256, U256,
};
pub use ethabi;
pub use near_sdk;
use near_sdk::{borsh, ext_contract};
use std::fmt;

/// Indicates an error with signature `Error(String)`. For more information see:
/// * https://docs.soliditylang.org/en/v0.8.18/control-structures.html#assert-and-require
/// * https://www.4byte.directory/signatures/?bytes4_signature=0x08c379a0
pub const SOLIDITY_ERROR_SELECTOR: [u8; 4] = [8, 195, 121, 160];

pub fn parse_address(input: &str) -> Result<Address, AddressError> {
    let input = input.strip_prefix("0x").unwrap_or(input);
    Address::decode(input)
}

pub fn parse_u256_base10(input: &str) -> Result<U256, ParseU256Error> {
    U256::from_dec_str(input).map_err(|e| ParseU256Error {
        msg: format!("{e:?}"),
    })
}

pub fn parse_evm_revert_message(input: &[u8]) -> String {
    if input.len() < 4 {
        return format!("0x{}", hex::encode(input));
    }

    let decoded = if input[0..4] == SOLIDITY_ERROR_SELECTOR {
        try_abi_parse_revert_message(input)
    } else {
        None
    };

    decoded.unwrap_or_else(|| format!("0x{}", hex::encode(input)))
}

fn try_abi_parse_revert_message(input: &[u8]) -> Option<String> {
    ethabi::decode(&[ethabi::ParamType::String], &input[4..])
        .ok()?
        .pop()?
        .into_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseU256Error {
    pub msg: String,
}

impl fmt::Display for ParseU256Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse U256 Error: {}", self.msg)
    }
}

impl std::error::Error for ParseU256Error {}

#[ext_contract(aurora_contract)]
pub trait AuroraInterface {
    #[result_serializer(borsh)]
    fn call(&mut self, #[serializer(borsh)] args: CallArgs) -> SubmitResult;
}
