use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::{near_bindgen, AccountId, PromiseOrValue};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct FtRefund;

#[near_bindgen]
impl FungibleTokenReceiver for FtRefund {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        black_hole(sender_id);
        if &msg == "refund" {
            PromiseOrValue::Value(amount)
        } else {
            PromiseOrValue::Value(0.into())
        }
    }
}

#[inline]
fn black_hole<T>(_t: T) {}
