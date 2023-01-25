use aurora_sdk::{
    ethabi, near_sdk, Address, CallArgs, FunctionCallArgsV1, SubmitResult, TransactionStatus, U256,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U64;
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Promise};

const DEFAULT_FEE: u64 = 500;
const EXACT_OUTPUT_SINGLE_SELECTOR: [u8; 4] = [219, 62, 33, 152];

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct UniswapProxy {
    aurora: AccountId,
    uniswap: Address,
}

#[near_bindgen]
impl UniswapProxy {
    #[init]
    pub fn new(aurora: AccountId, uniswap_address: String) -> Self {
        Self {
            aurora,
            uniswap: aurora_sdk::parse_address(&uniswap_address).unwrap(),
        }
    }

    pub fn exact_output_single(&self, params: SerializableExactOutputSingleParams) -> Promise {
        let params: ExactOutputSingleParams = params.try_into().unwrap();
        let evm_token = ethabi::Token::Tuple(vec![
            ethabi::Token::Address(params.token_in.raw()),
            ethabi::Token::Address(params.token_out.raw()),
            ethabi::Token::Uint(params.fee.into()),
            ethabi::Token::Address(params.recipient.raw()),
            ethabi::Token::Uint(params.deadline),
            ethabi::Token::Uint(params.amount_out),
            ethabi::Token::Uint(params.amount_in_max),
            ethabi::Token::Uint(params.price_limit),
        ]);
        let evm_input = ethabi::encode(&[evm_token]);
        let aurora_call_args = CallArgs::V1(FunctionCallArgsV1 {
            contract: self.uniswap,
            input: [
                EXACT_OUTPUT_SINGLE_SELECTOR.as_slice(),
                evm_input.as_slice(),
            ]
            .concat(),
        });
        aurora_sdk::aurora_contract::ext(self.aurora.clone())
            .with_unused_gas_weight(3)
            .call(aurora_call_args)
            .then(Self::ext(env::current_account_id()).parse_exact_output_single_result())
    }

    #[private]
    pub fn parse_exact_output_single_result(
        &self,
        #[serializer(borsh)]
        #[callback_unwrap]
        result: SubmitResult,
    ) -> ExactOutputSingleResult {
        match result.status {
            TransactionStatus::Succeed(bytes) => {
                let amount_in = U256::from_big_endian(&bytes);
                ExactOutputSingleResult {
                    amount_in: amount_in.to_string(),
                }
            }
            TransactionStatus::Revert(bytes) => {
                let error_message =
                    format!("Revert: {}", aurora_sdk::parse_evm_revert_message(&bytes));
                env::panic_str(&error_message)
            }
            other => env::panic_str(&format!("Aurora Error: {other:?}")),
        }
    }
}

struct ExactOutputSingleParams {
    pub token_in: Address,
    pub token_out: Address,
    pub fee: u64,
    pub recipient: Address,
    pub deadline: U256,
    pub amount_out: U256,
    pub amount_in_max: U256,
    pub price_limit: U256,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExactOutputSingleResult {
    amount_in: String,
}

impl TryFrom<SerializableExactOutputSingleParams> for ExactOutputSingleParams {
    type Error = ParseParamsError;

    fn try_from(value: SerializableExactOutputSingleParams) -> Result<Self, Self::Error> {
        let token_in = parse_address(&value.token_in)?;
        let token_out = parse_address(&value.token_out)?;
        let fee = value.fee.map(|x| x.0).unwrap_or(DEFAULT_FEE);
        let recipient = parse_address(&value.recipient)?;
        let deadline = value
            .deadline
            .as_deref()
            .map(parse_u256)
            .unwrap_or(Ok(U256::MAX))?;
        let amount_out = parse_u256(&value.amount_out)?;
        let amount_in_max = value
            .amount_in_max
            .as_deref()
            .map(parse_u256)
            .unwrap_or_else(|| amount_out.checked_mul(100.into()).ok_or(ParseParamsError))?;
        let price_limit = value
            .price_limit
            .as_deref()
            .map(parse_u256)
            .unwrap_or(Ok(U256::zero()))?;

        Ok(Self {
            token_in,
            token_out,
            fee,
            recipient,
            deadline,
            amount_out,
            amount_in_max,
            price_limit,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializableExactOutputSingleParams {
    pub token_in: String,
    pub token_out: String,
    #[serde(default)]
    pub fee: Option<U64>,
    pub recipient: String,
    #[serde(default)]
    pub deadline: Option<String>,
    pub amount_out: String,
    #[serde(default)]
    pub amount_in_max: Option<String>,
    #[serde(default)]
    pub price_limit: Option<String>,
}

#[derive(Debug)]
struct ParseParamsError;

#[inline]
fn parse_address(input: &str) -> Result<Address, ParseParamsError> {
    aurora_sdk::parse_address(input).map_err(|_| ParseParamsError)
}

#[inline]
fn parse_u256(input: &str) -> Result<U256, ParseParamsError> {
    aurora_sdk::parse_u256_base10(input).map_err(|_| ParseParamsError)
}
