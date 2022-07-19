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
    fn log_transfer(receiver: String , amount: U128, token: AccountId, sender: AccountId);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct TippingBot {
    
    pub owner_id: AccountId,

}

impl Default for TippingBot {
    fn default() -> Self {
        panic!("Should be initialized before usage")
    }
}

#[near_bindgen]
impl TippingBot {
    #[init]
    pub fn new(owner_id: AccountId, transfer_fee: U128) -> Self {
        assert!(env::is_valid_account_id(owner_id.as_bytes()), "Invalid owner account");
        assert!(!env::state_exists(), "Already initialized");
        
        Self {
            owner_id: owner_id,
        }
    }

    #[payable]
    pub fn transfer_payment(&mut self, receiver: AccountId) -> Promise {

        let sender_id = env::predecessor_account_id();
        let amount = env::attached_deposit();
    
        Promise::new(receiver.clone()).transfer(amount).then(
            ext_self::log_transfer(receiver, U128(amount),"$NEAR".to_string(), sender_id,
                &env::current_account_id(), 0, BASE_GAS)
        )
    }

    //receiver: String , amount: U128, token: AccountId, sender: AccountId

    pub fn ft_on_transfer(&mut self, sender_id: String, amount: U128, msg: String) -> String {
    
        let parsed_message: Value = serde_json::from_str(&msg).expect("No message was parsed");
        let receiver = parsed_message["receiver"].as_str().expect("No receiver was parsed on the message");

        token_contract::ft_transfer(receiver.to_string(), U128(amount.clone()), "memo".to_string(), 
                            &env::predecessor_account_id(), 1, BASE_GAS
        ).then(
            ext_self::log_transfer(receiver.to_string(), amount,env::predecessor_account_id(),
            sender_id,
                &env::current_account_id(), 0, BASE_GAS)
        );
        "0".to_string()
    }

    #[private]
    pub fn log_transfer(receiver: String , amount: U128, token: AccountId, sender: AccountId) {

            assert_eq!(env::promise_results_count(), 1, "ERR_TOO_MANY_RESULTS");
            match env::promise_result(0) {
                PromiseResult::NotReady => unreachable!(),
                PromiseResult::Successful(_val) => {
                    log!("{}", &json!({
                        "standard": "The-Supah-Tipping-bot",
                        "version": "1.0.0",
                        "event": "transfer",
                        "data": {
                            "sender": sender,
                            "receiver": receiver,
                            "token": token,
                            "amount":amount,
                        }
                    }).to_string());
                },
                PromiseResult::Failed => env::panic(b"ERR_CALL_FAILED"),
            }
    }
}

//----------------------------------- TEST -------------------------------------------------

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext, Balance};

    use super::*;
    use std::convert::TryFrom;

    pub const TOTAL_SUPPLY: Balance = 1_000 ;
    pub const CONTRACT_ACCOUNT: &str = "contract.testnet";
    pub const TOKEN_ACCOUNT: &str = "token.testnet";
    pub const SIGNER_ACCOUNT: &str = "signer.testnet";
    pub const OWNER_ACCOUNT: &str = "owner.testnet";

    // mock the context for testing, notice "signer_account_id" that was accessed above from env::
  pub fn get_context(input: Vec<u8>, is_view: bool, attached_deposit: u128, account_balance: u128, signer_id: AccountId) -> VMContext {
    VMContext {
        current_account_id: CONTRACT_ACCOUNT.to_string(),
        signer_account_id: signer_id.clone(),
        signer_account_pk: vec![0, 1, 2],
        predecessor_account_id: signer_id.clone(),
        input,
        block_index: 0,
        block_timestamp: 0,
        account_balance,
        account_locked_balance: 0,
        storage_usage: 0,
        attached_deposit,
        prepaid_gas: 10u64.pow(18),
        random_seed: vec![0, 1, 2],
        is_view,
        output_data_receivers: vec![],
        epoch_height: 19,
    }
  }

  pub fn get_test_meta() -> FungibleTokenMetadata{
    FungibleTokenMetadata {
        spec: FT_METADATA_SPEC.to_string(),
        name: "Example NEAR fungible token".to_string(),
        symbol: "EXAMPLE".to_string(),
        icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
        reference: None,
        reference_hash: None,
        decimals: 24,
    }
      
  }

  pub fn init_contract() -> Contract{
    Contract {
        token: FungibleToken::new(b"a".to_vec()),
        metadata: LazyOption::new(b"m".to_vec(), Some(&get_test_meta()))
    }
  }


    #[test]
    fn test_new() {
        let mut context = get_context(vec!(), false, 0, 0, OWNER_ACCOUNT.to_string()); // vec!() -> da pra inicializar assim, tem otimizacao ( macro vec)
    
        testing_env!(context);
        let contract = Contract::new(OWNER_ACCOUNT.to_string(), TOTAL_SUPPLY.into(), get_test_meta());
        let contract_metadata = contract.metadata.get().unwrap();
         
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of( ValidAccountId::try_from(OWNER_ACCOUNT).unwrap() ).0, TOTAL_SUPPLY);
        assert_eq!(contract_metadata.spec, get_test_meta().spec)
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let mut context = get_context(vec!(), false, 0, 0, OWNER_ACCOUNT.to_string()); 
        testing_env!(context);
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(vec!(), false, 1, 0, SIGNER_ACCOUNT.to_string());
        testing_env!(context);

        let mut contract = init_contract();

        //registring owner 
        contract.token.internal_register_account(&OWNER_ACCOUNT.to_string());
        contract.token.internal_register_account(&SIGNER_ACCOUNT.to_string());
        contract.token.internal_deposit(&SIGNER_ACCOUNT.to_string(), TOTAL_SUPPLY);      
        
        let transfer_amount = 10;

        contract.ft_transfer(ValidAccountId::try_from(OWNER_ACCOUNT).unwrap(), U128(transfer_amount),None );

        assert_eq!(contract.ft_balance_of(ValidAccountId::try_from(SIGNER_ACCOUNT).unwrap()).0, (TOTAL_SUPPLY - transfer_amount));
        assert_eq!(contract.ft_balance_of(ValidAccountId::try_from(OWNER_ACCOUNT).unwrap()).0, transfer_amount);
    }


}

