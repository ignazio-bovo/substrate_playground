use codec::{Decode, Encode}; 
use frame_support::{
    decl_event, decl_storage, decl_module,
    dispatch::{DispatchResult, Vec},
    StorageValue,
    ensure,
};

use frame_system::ensure_signed;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{
    crypto::Public as _, 
    H256, 
    H512,
    //sr25519::{Public,Signature},
};

use sp_std::collections::btree_map::BTreeMap;
use sp_runtime::{
    traits::{BlakeTwo256, Hash, SaturatedConversion}, 
    transaction_validity::{TransactionLongevity, ValidTransaction},
};

// this entire module that we are building implement this Config trait
pub trait Config: frame_system::Config {
    type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;
}

decl_storage! {
    trait Store for Module<T: Config> as Utxo {
         // specifies how the initial configuration is build
         UtxoStore build(|config: &GenesisConfig| {
            config.genesis_utxos // need to add the genesis_utxos member to the genesis block
                .iter()
                .cloned()
                .map(|u| (BlakeTwo256::hash_of(&u), u))
                .collect::<Vec<_>>()
        }): map hasher(identity) H256 => Option<TransactionOutput>;
        pub RewardTotal get(reward_total): Value;
    }

    // Init state
    add_extra_genesis {
     config(genesis_utxos): Vec<TransactionOutput>;
    }
}

decl_module! {
    // function that will do the state transitioning eg the function that allows
    // to spend a utxo token
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = 1_000]
        pub fn spend(_origin, transaction: Transaction) -> DispatchResult {
            
            // 1. verify first : transaction validity
            let transaction_validity = Self::validate_transaction(&transaction)?;

            // 2. Write transation to state
            Self::update_storage(&transaction)?;

            // 3. Wolud be nice to emit some sort of event signalling succssful transaction
            Self::deposit_event(Event::TransactionSuccess(transaction));

            Ok(())
        }

        fn on_finalize() {
            // send tips to validators
            let auth: Vec<_> = Aura::authorities().iter().map(|x| {
                let r: &Public = x.as_ref();
                r.0.into()
            }).collect();
            Self::disperse_reward(&auth);
        }
    }
}


decl_event! (
    // when anything happens events can be emitted 
    // what's expected is a series of variants that we are expected to emite
	pub enum Event {
		/// Transaction was executed successfully
		TransactionSuccess(Transaction),
		/// Rewards were issued. Amount, UTXO hash.
		RewardsIssued(Value, H256),
		/// Rewards were wasted
		RewardsWasted,
	}
);

// data structures used 
pub type Value = u128;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, PartialOrd, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct Transaction {
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, PartialOrd, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct TransactionInput {
    // reference to a utxo to be spent
    pub outpoint: H256,

    // proof that the transaction owner is authorized 
    pub sigscript:H512,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, PartialOrd, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct TransactionOutput {
    // money to be send
    pub value: Value, 

    // hash([value, publicKey]) 
    pub pubkey: H256,
}

// implement helper function for the trait Config 
impl<T: Config> Module<T> {
    fn update_storage(transaction: &Transaction, reward: Value) -> DispatchResult {
        let new_total = <RewardTotal>::get()
            .checked_add(reward)
            .ok_or("reward overflow")?;
        <RewardTotal>::put(new_total);
        // transaction contains two vector: possibly very expensive a pass by value (copying)
        // since we don't want to only read from transaction we may want to borrow from it 
        // (aka pass by const reference in C++)

        //1. Remove input from UtxoStore 
        for input in &transaction.inputs {
            <UtxoStore>::remove(input.outpoint); // remove element using key == (hash value)
        }
        
        //2. Create new UTXOs in UtxoStore
        let mut index: u64 = 0;
        for output in &transaction.output {
            let hash = BlakeTwo256::hash_of((&transaction.encode(), index));
            index = index.checked_add(1).ok_or("output index overflow")?;
            <UtxoStore>::insert(hash,output);
        }
          
        Ok(())

    }
    fn disperse_reward(authorities: &[H256]) {
        //1. divide reward fairly 
        let reward = <RewardTotal>::take();// deletes the previous reward total
        let share_value: Value = reward
            .checked_div(authorities.len() as Value)
            .ok_or("No Authorities")
            .unwrap();

        if share_value == 0 { return }

        let remainder = reward
            .checked_sub(share_value + authorities.len() as Value)
            .ok_or("Sub underflow")
            .unwrap();
        <RewardTotal>::put(remainder);

        //2. create utxo per validator
        for authority in authorities {
            let utxo = TransactionOutput{
                value: share_value, 
                pubkey: *authority,
            };
            let hash = BlakeTwo256::hash_of(&(&utxo, 
                                              <system::Module<T>>::block_number()
                                              .saturated_into::<u64>()));
            if !<UtxoStore>::contains_key(hash) {
                <UtxoStore>::insert(hash, utxo);
                sp_runtime::print("Transaction reward sent");
                sp_runtime::print(hash.as_fixed_bytes() as &[u8]);
            } else {
                sp_runtime::print("Transaction reward");
            }
        }
         
        //3. write utxos to UtxoStore
    }
    // in order to prevent a replay attack : malitious nodes can observe recurring 
    // transactioon and copy utxo signatures to spend Alice utxos . instead of hashing 

}
