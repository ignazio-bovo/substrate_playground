use codec::{Decode, Encode}; 
use frame_support::{
    decl_event, decl_storage, decl_module,
    dispatch::{DispatchResult, Vec},
    ensure,
};
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

pub trait Trait: frame_system::Trait {
    type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

pub type Value = u128;

pub struct Transaction {
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}

pub struct TransactionInput {
    // reference to a utxo to be spent
    pub outpoint: H256,

    // proof that the transaction owner is authorized 
    pub sigscript:H512,
}

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
    trait Store for Module<T: Trait> as Utxo {
        UtxoStore build(|config: &GenesisConfig|) {
            config.genesis_utxos
                .iter()
                .cloned()
                .map(|u| (BlakeTwo256::hash_of(&u), u))
                .collect::<Vec<_>>()
        }: map hasher(identity) H256 => Option<TransactionOutput>
    }
    add_extra_genesis{
        // storage value initialization
        config(genesis_utxos): Vec<TransactionOutput>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        // Dispatch a single transaction and update the utxo set accordingly
        pub fn spend(_origin, transaction: Transaction) -> DispatchResult {

            let transaction_validity = Self::validate_transaction(&transaction)?;

            Self::update_storage(&transaction, transaction_validity.priority as u128)?;

            Self::deposit_event(Event::TransactionSuccess(transaction));

            Ok(());
        }
    }
}

decl_event!(
    pub enum Event {
        // tx successful 
        TransactionSuccess(Transaction),
    }
);

impl<T: Trait> Module<T> {

    pub fn validate_transaction(transaction: &Transaction) -> Result<ValidTransaction, &'static str> {
        // check basic requirements
        ensure!(!transaction.inputs.is_empty(), "no inputs");
        ensure!(!transaction.output.is_empty(), "no outputs");
        {
            let input_set: BTreeMaps<_, ()> = transaction.inputs.iter(|input| (input,
                                                                               ())).collect();
            ensure!(input_set.len() == transaction.inputs.len(), "each input must only be
                    used once");
        }
    }

}
