use codec::{Decode, Encode}; 
use frame_support::{
    decl_event, decl_storage, decl_module,
    dispatch::{DispatchResult, Vec},
    ensure,
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{
    crypto::Public as _, 
    H256, 
    H512,
    //sr25519::{Public,Signature},
};
//use sp_std::collections::btree_map::BTreeMap;
use sp_runtime::{
    traits::{BlakeTwo256, Hash, SaturatedConversion}, 
    //transaction_validity::{TransactionLongevity, ValidTransaction},
};

pub trait Config: frame_system::Config {
    type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;
}

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

// how the state is stored on chain ?

decl_storage! {
    // declaration: 
    // * Store trait generated associating each storage item to the Module 
    // * Utxo:  prefix used for storage items of this module
    trait Store for Module<T: Config> as Utxo {
        UtxoStore build(|config: &GenesisConfig| {
            config.genesis_utxos
                .iter()
                .cloned()
                .map(|u| (BlakeTwo256::hash_of(&u), u))
                .collect::<Vec<_>>()
        }): map hasher(identity) H256 => Option<TransactionOutput>
    }
    add_extra_genesis{
        // storage value initialization
        config(genesis_utxos): Vec<TransactionOutput>;
    }
}
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        #[weight = 0]
        fn my_long_function(_origin) -> DispatchResult {
            Ok(())
        }
    }
}
decl_event!(
	pub enum Event {
		/// Transaction was executed successfully
		TransactionSuccess(Transaction),
		/// Rewards were issued. Amount, UTXO hash.
		RewardsIssued(Value, H256),
		/// Rewards were wasted
		RewardsWasted,
	}
);

