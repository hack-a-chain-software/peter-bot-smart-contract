use near_sdk::{ borsh };
use borsh::{ BorshDeserialize, BorshSerialize };
use near_sdk::{
    env, near_bindgen, AccountId, Promise,
    log, Gas, PromiseResult,
    json_types::{ U128 },
    utils::assert_one_yocto, ext_contract
};
pub use near_sdk::serde_json::{self, json, Value};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc = near_sdk::wee_alloc::WeeAlloc::INIT;

// const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const FRACTIONAL_BASE: u128 = 10_000;
pub const BASE_GAS: Gas = 5_000_000_000_000;


#[ext_contract(token_contract)]
trait FungibleToken {
    fn ft_transfer(receiver_id: String, amount: U128, memo: String);
}

#[ext_contract(ext_self)]
trait LogInfo {
    fn log_transfer(receiver: String, burner: String, full_amount: U128, transfered_amount: U128,
        fee_amount: U128, burn_amount: U128, token: AccountId, sender: AccountId);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct PeterBot {
    
    pub owner_id: AccountId,
    pub transfer_fee: u128 // X / 10000

}

impl Default for PeterBot {
    fn default() -> Self {
        panic!("Should be initialized before usage")
    }
}

#[near_bindgen]
impl PeterBot {
    #[init]
    pub fn new(owner_id: AccountId, transfer_fee: U128) -> Self {
        assert!(env::is_valid_account_id(owner_id.as_bytes()), "Invalid owner account");
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner_id: owner_id,
            transfer_fee: transfer_fee.0
        }
    }

    #[payable]
    pub fn transfer_payment(&mut self, receiver: AccountId) -> Promise {

        let sender_id = env::predecessor_account_id();
        let amount = env::attached_deposit();
        let receiver_amount = ( amount * (FRACTIONAL_BASE - self.transfer_fee) ) / FRACTIONAL_BASE;
        let fee_amount = amount - receiver_amount;

        Promise::new(receiver.clone()).transfer(receiver_amount).then(
            ext_self::log_transfer(receiver, "null".to_string(), U128(amount), U128(receiver_amount),
                U128(fee_amount), U128(0), "$NEAR".to_string(), sender_id,
                &env::current_account_id(), 0, BASE_GAS)
        )
    }

    pub fn ft_on_transfer(&mut self, sender_id: String, amount: U128, msg: String) -> String {
               
        let receiver_amount = ( amount.0 * (FRACTIONAL_BASE - (self.transfer_fee * 2) ) ) / FRACTIONAL_BASE;
        let burn_amount = ( amount.0 * (self.transfer_fee) ) / FRACTIONAL_BASE;
        let fee_amount = amount.0 - receiver_amount - burn_amount;
        let parsed_message: Value = serde_json::from_str(&msg).expect("1");
        let receiver = parsed_message["receiver"].as_str().expect("2");
        let burner = parsed_message["burner"].as_str().expect("3");


        token_contract::ft_transfer(receiver.to_string(), U128(receiver_amount), "memo".to_string(), 
                            &env::predecessor_account_id(), 1, BASE_GAS
        ).then(
            token_contract::ft_transfer(burner.to_string(), U128(burn_amount), "memo".to_string(), 
                            &env::predecessor_account_id(), 1, BASE_GAS)
        ).then(
            ext_self::log_transfer(receiver.to_string(), burner.to_string(), amount, U128(receiver_amount),
                U128(fee_amount), U128(burn_amount), env::predecessor_account_id(), sender_id,
                &env::current_account_id(), 0, BASE_GAS)
        );
        "0".to_string()
    }

    #[private]
    pub fn log_transfer(receiver: String, burner: String, full_amount: U128, transfered_amount: U128,
        fee_amount: U128, burn_amount: U128, token: AccountId, sender: AccountId) {

            assert_eq!(env::promise_results_count(), 1, "ERR_TOO_MANY_RESULTS");
            match env::promise_result(0) {
                PromiseResult::NotReady => unreachable!(),
                PromiseResult::Successful(_val) => {
                    log!("{}", &json!({
                        "standard": "Peter-the-wire-bot",
                        "version": "1.0.0",
                        "event": "transfer",
                        "data": {
                            "sender": sender,
                            "receiver": receiver,
                            "burner": burner,
                            "token": token,
                            "full_amount": full_amount,
                            "transfered_amount": transfered_amount,
                            "fee_amount": fee_amount,
                            "burn_amount": burn_amount,
                        }
                    }).to_string());
                },
                PromiseResult::Failed => env::panic(b"ERR_CALL_FAILED"),
            }
    }
    
    #[payable]
    pub fn change_fee(&mut self, new_fee: U128) -> U128 {
        assert!(env::predecessor_account_id() == self.owner_id, "Only owner can call this function");
        assert_one_yocto();
        self.transfer_fee = new_fee.0;
        U128(self.transfer_fee)
    }

    pub fn get_fee(&self) -> U128 {
        U128(self.transfer_fee)
    }

    #[payable]
    pub fn withdraw_funds(&mut self, quantity: U128) -> Promise {
        assert!(env::predecessor_account_id() == self.owner_id, "Only owner can call this function");
        assert_one_yocto();
        Promise::new(self.owner_id.clone()).transfer(quantity.0)
    }

}

