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
    }
}

