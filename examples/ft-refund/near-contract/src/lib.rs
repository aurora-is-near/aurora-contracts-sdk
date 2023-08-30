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
        // The `sender_id` parameter is intentionally not used in this implementation.
        // Typically you would prefix the variable name with an underscore to suppress
        // the compiler warning about an unused variable, but that does not work here
        // because the variable names are used in the `near_bindgen` generated code
        // to parse the JSON arguments passed to this function and we still want the
        // field to be called `sender_id` there, not `_sender_id`.
        // For this reason we have a no-op function that will take any argument to make
        // it "used" from the perspective of the compiler.
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
