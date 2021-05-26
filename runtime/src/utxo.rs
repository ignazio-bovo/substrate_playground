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
    sr25519::{Public,Signature},
};

use sp_std::collections::btree_map::BTreeMap;
use sp_runtime::{
    traits::{BlakeTwo256, Hash, SaturatedConversion}, 
    transaction_validity::{TransactionLongevity, ValidTransaction},
};

use frame_support::sp_io;

// this entire module that we are building implement this 
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
        pub RewardTotal get(fn reward_total): Value;
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
            let reward = Self::validate_transaction(&transaction)?;

            // 2. Write transation to state
            Self::update_storage(&transaction, reward)?;

            // 3. Wolud be nice to emit some sort of event signalling succssful transaction
            Self::deposit_event(Event::TransactionSuccess(transaction));

            Ok(())
        }

        //fn on_finalize() {
            // send tips to validators
            //Self::disperse_reward(&auth);
        //}
    }
}


decl_event! (
    // when anything happens events can b)e emitted 
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
#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct TransactionInput {
    // reference to a utxo to be spent
    pub outpoint: H256,

    // proof that the transaction owner is authorized 
    pub sigscript:H512,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct TransactionOutput {
    // money to be send
    pub value: Value, 

    // hash([value, publicKey]) 
    pub pubkey: H256,
}

// implement helper function for the trait Config 
impl<T: Config> Module<T> {
    pub fn get_simple_transaction(transaction: &Transaction) -> Vec<u8> {
        let mut trx = transaction.clone();
        for input in trx.inputs.iter_mut() {
            input.sigscript = H512::zero(); 
        }
        trx.encode()

    }
    fn validate_transaction(transaction: &Transaction) -> Result<Value, &'static str> {
        // ensure that tx has valid inputs and outputs 
        ensure!(!transaction.inputs.is_empty(), "no_inputs");
        ensure!(!transaction.outputs.is_empty(), "no_inputs");

        // one time use heap variables
        {
            let input_set: BTreeMap<_, ()> = transaction.inputs.iter().map(|input| (input, ()))
                .collect();
            ensure!(input_set.len() == transaction.inputs.len(), "each input must only be used once");
        }

        {
            let output_set: BTreeMap<_, ()> = transaction.inputs.iter().map(|input| (input, ()))
                .collect();
            ensure!(output_set.len() == transaction.inputs.len(), "each input must only be used once");
        }
        
        let simple_transaction = Self::get_simple_transaction(transaction);
        let mut total_input: Value = 0;
        let mut total_output: Value = 0; 

        // verify input tx 
        for input in transaction.inputs.iter() {
            if let Some(input_utxo) = <UtxoStore>::get(&input.outpoint) {
                ensure!(sp_io::crypto::sr25519_verify(
                        &Signature::from_raw(*input.sigscript.as_fixed_bytes()),
                        &simple_transaction, 
                        &Public::from_h256(input_utxo.pubkey)
                                                     ), 
                    "signature must be valid");
                total_input = total_input.checked_add(input_utxo.value).ok_or("input value overflow")?;
            } else {
                // TODO
            }
        }

        let mut output_index: u64 = 0; 
        for output in transaction.outputs.iter() {
            ensure!(output.value > 0, " output value must be nonzero");
            let hash = BlakeTwo256::hash_of(&(&transaction.encode(), output_index));
            output_index = output_index.checked_add(1).ok_or("output inedx overflow")?;
            ensure!(!<UtxoStore>::contains_key(hash), "output already exists"); 
            total_output = total_output.checked_add(output.value).ok_or("output value overflow")?;
            
        }

        ensure!( total_input >= total_output, "output value must be <= input value"); 
        let reward = total_input.checked_sub(total_output).ok_or("reward overflow")?;
        Ok(reward)

    }
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
        for output in &transaction.outputs {
            let hash = BlakeTwo256::hash_of(&(&transaction.encode(), index));
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
                                              <frame_system::Module<T>>::block_number()
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

//// TESTS for this module
#[cfg(test)]
mod tests {
	use super::*; // use the whole Module above as a library // use the whole Module above as a library

	use frame_support::{assert_ok, assert_noop, impl_outer_origin, parameter_types, weights::Weight};
	use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
	use sp_keystore::testing::KeyStore;//, SR25519};
	use sp_keystore::KeystoreExt;

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
			pub const BlockHashCount: u64 = 250;
			pub const MaximumBlockWeight: Weight = 1024;
			pub const MaximumBlockLength: u32 = 2 * 1024;
			pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl frame_system::Config for Test {
		type BaseCallFilter = ();
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type DbWeight = ();
		type BlockExecutionWeight = ();
		type ExtrinsicBaseWeight = ();
		type MaximumExtrinsicWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
		type ModuleToIndex = ();
		type AccountData = ();
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
	}

	impl Config for Test {
		type Event = ();
		type BlockAuthor = ();
		type Issuance = ();
	}
