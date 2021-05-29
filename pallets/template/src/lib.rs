use codec::{Decode, Encode};
use sp_core::{
    crypto::Public as _,
    H256,
    H512,
    sr25519::{Public, Signature},
};

use sp_std::collections::btree_map::BTreeMap;

use sp_io::crypto::sr25519_verify;

use sp_runtime::{
    traits::{BlakeTwo256,Hash},
    transaction_validity::{ValidTransaction,TransactionLongevity},
};

#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};
// crypto signature scheme to certify transactions

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>

use frame_support::{decl_module, decl_storage, decl_event, decl_error,
    dispatch::{DispatchResult, Vec}, ensure};

use frame_system::ensure_signed;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub type Value = u128;

//#[cfg(feature = "runtime-benchmarks")]
//mod benchmarking;
/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config {
	/// Because this pallet emits events, it depends on the runtime's definition of an event.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    // A source to determine he issuance portion of the block reward 
    //type Issuance: Issuance<<Self as frame_system::Config>::BlockNumber, Value>;

    // a source to determine the block author
    //type BlockAuthor: BlockAuthor;
}

#[cfg_attr(feature="std", derive(Serialize, Deserialize))] 
#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct Utxo { // UTXO
    pub value: Value, 
    pub pubScript: H256,
}

#[cfg_attr(feature="std", derive(Serialize, Deserialize))] 
#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct TxInput {
    pub utxo_ref: H256, 
    pub scriptSig: H512,
}

#[cfg_attr(feature="std", derive(Serialize, Deserialize))] 
#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash, Debug)]
pub struct Transaction {
    pub output: Vec<Utxo>,
    pub input: Vec<TxInput>,
}

// The pallet's runtime storage items.
// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
	// A unique name is used to ensure that the pallet's storage items are isolated.
	// This name may be updated, but each pallet in the runtime must use a unique name.
	// ---------------------------------vvvvvvvvvvvvvv
    // Module will have as attributes the storage items specified below (as structs in order to be
    // type-safe)
    // In order to configure its genesis state we add a special attribute to the Module<T> 
    // of type GenesisConfig which will be a struct whose attribute can be initialized with 
    // the config macro.
	trait Store for Module<T: Config> as TemplateModule {
		// Learn more about declaring storage items:
		// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
        // initial value for this storage item set using the build closure that must always
        // have one parameter of type GenesisConfig
        UtxoSet build(|config: &GenesisConfig| {
            config.genesis_utxos 
                .iter()
                .cloned()
                .map(|u| (BlakeTwo256::hash_of(&u), u))
                .collect::<Vec<_>>()
        }): map hasher(identity) H256 => Option<Utxo>;
        pub RewardTotal get(fn reward_total): Value;
	}
    add_extra_genesis {
        config(genesis_utxos): Vec<Utxo>;
    }
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, AccountId),
        TransactionSuccess(Transaction),
        RewardIssued(Value, H256),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Config> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}
}

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {

		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;

        // spend function which construct a transaction 
        #[weight = 1_000]
        fn spend(_origin, tx: Transaction) -> DispatchResult {
            let transaction_validity = Self::validate_transaction(&tx)?;
            ensure!(transaction_validity.requires.is_empty(), "missing inputs"); 

            Self::update_storage(&tx, transaction_validity.priority as Value)?;

            Self::deposit_event(RawEvent::TransactionSuccess(tx));

            Ok(())
        }

        // handler called by the system on block finalization 
        //fn on_finalize() {
            //match T::BlockAuthor::block_author() {
                //None => Self::deposit_event(Event::RewardsWasted),
                //Some(author) => Self::disperse_reward(&author),
            //}           
        //}
	}
}

impl <T: Config> Module<T> {
    pub fn validate_transaction(tx: &Transaction) -> Result<ValidTransaction,&'static str> {
        // Ensures that:
        // - 1  inputs and outputs are not empty
        // - 2 all inputs match to existing, unspent and unlocked outputs
        // - 3 each input is used exactly once
        // - 4 each output is defined exactly once and has nonzero value
        // - 5 total output value must not exceed total input value
        // - 6 new outputs do not collide with existing ones
        // - 7 sum of input and output values does not overflow
        // - 8 provided signatures are valid
        // - 9 transaction outputs cannot be modified by malicious nodes

        // 1
        ensure!(!tx.input.is_empty(), "no inputs");
        ensure!(!tx.output.is_empty(), "no outputs");
        
        // 3
        {
            let input_set: BTreeMap<_, ()> = tx.input.iter().map(|input| (input, ())).collect();
            ensure!(input_set.len() == tx.input.len(), "each input must be used only once")
        }

        // 4
        {
            let output_set: BTreeMap<_, ()> = tx.output.iter().map(|output| (output, ())).collect();
            ensure!(output_set.len() == tx.output.len(), "each output must be used only once")
        }

        // total in/out value in satoshis
        let mut total_input: Value = 0;
        let mut total_output: Value = 0;
        let mut output_index: u64 = 0;
        let simple_tx = Self::get_simple_transaction(tx);

        // Variables sent to transaction pool
        let mut missing_utxos = Vec::new();
        let mut new_utxos = Vec::new();
        let mut reward: Value = 0;

        for input in tx.input.iter() {
            if let Some(input_utxo) = <UtxoSet>::get(&input.utxo_ref) {
                // 8
                ensure!(sr25519_verify(
                        &Signature::from_raw(*input.scriptSig.as_fixed_bytes()),
                        &simple_tx,
                        &Public::from_h256(input_utxo.pubScript)),
                        "input signature verification failed");
                total_input = total_input.checked_add(input_utxo.value).ok_or("input value overflow")?;
            } else {
                missing_utxos.push(input.utxo_ref.clone().as_fixed_bytes().to_vec());
            }

        }

        for output in tx.output.iter() {
            // 4
            ensure!(output.value > 0, "output value must be non zero");
            let hash = BlakeTwo256::hash_of(&(&tx.encode(), output_index)); // prevent replay attacks
            output_index = output_index.checked_add(1).ok_or("output index overflow")?;
            // 6 
            ensure!(!<UtxoSet>::contains_key(hash), "output already exists");
            total_output = total_output.checked_add(output.value).ok_or("total output overflow")?;
            new_utxos.push(hash.as_fixed_bytes().to_vec());
        }

        // race conditions
        if missing_utxos.is_empty() {
            ensure!( total_input >= total_output, "output value muste not exceed input value" ); 
            reward = total_input.checked_sub(total_output).ok_or("reward underflow")?;
        }
        Ok(ValidTransaction {
            requires: missing_utxos, 
            provides: new_utxos, 
            priority: reward as u64, 
            longevity: TransactionLongevity::max_value(),
            propagate: true,
        })
    } 
    pub fn update_storage(transaction: &Transaction, priority: Value) -> DispatchResult {

        // calculate new reward total 
        let new_total = <RewardTotal>::get()
            .checked_add(priority)
            .ok_or("reward overflow")?;
        <RewardTotal>::put(new_total);

        for input in transaction.input.iter() {
            <UtxoSet>::remove(input.utxo_ref);
        }

        let mut index: u64 = 0;
        for output in transaction.output.iter() {
            // adding a nonce to avoid replay attack
            let hash = BlakeTwo256::hash_of(&(&transaction.encode(), index)); 
            index = index.checked_add(1).ok_or("output index overflow")?; 
            <UtxoSet>::insert(hash, output);
        }

        Ok(())
    }

    pub fn get_simple_transaction(tx: &Transaction) -> Vec<u8> {
        let mut _tx = tx.clone();
        for input in _tx.input.iter_mut() {
            input.scriptSig = H512::zero();
        }
        _tx.encode()
    }

    fn dispense_reward(author: &Public) {
        let reward = RewardTotal::take() ;//+ T::Issuance::issuance(frame_system::Module::<T>::block_number());
        let utxo = Utxo {
            value: reward, 
            pubScript: H256::from_slice(author.as_slice()),
        };

        let temporary_fix: u64 = 0;
        let hash = BlakeTwo256::hash_of(&(&utxo,temporary_fix));
        <UtxoSet>::insert(hash, utxo); 
        Self::deposit_event(RawEvent::RewardIssued(reward, hash));
    }
}
