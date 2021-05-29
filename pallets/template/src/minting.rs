pub mod minting {
    #![cfg_attr(not(feature = "std"), no_std)] 

    // A trait for types that can provide the amount of issuance to ward to the block
    pub trait Issuance<BlockNumber, Balance> {
        fn issuance(block: BlockNumber) -> Balance;
    }

    // minimal implementations for when you don't actually want any issuance
    impl Issuance<u32, u128> for () {
        fn issuance(_block: u32) -> u128 {0}
    }

    impl Issuance<u64, u128> for () {
        fn issuance(_block: u64) -> u128 {0}
    }

    // A type that provides block issuance according to bitcoin's rules 
    // Initial issuance is 50 / block 
    // Issuance is cut in hal every 210,000 blocks

    pub struct BitcoinHalving;

    // no. of blocks between each halving
    const HALVING_INTERVAL: u32 = 210_000; 

    // per block issuance before any halvings
    const INITIAL_ISSUANCE: u32 = 50;

    impl Issuance<u32, u128> for BitcoinHalving {
        fn issuance(block: u32) -> u128 {
            let halvings = block / HALVING_INTERVAL;
            if halvings >= 64 { 
                return 0;
            }
            (INITIAL_ISSUANCE >> halvings).into()
        }
    }
}
