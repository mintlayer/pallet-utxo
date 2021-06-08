// #![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{BlakeTwo256, Hash};
    use primitive_types::{H256, H512};

    use core::marker::PhantomData;

    pub type Value = u128;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash, Default)]
    pub struct TransactionInput {
        pub(crate) outpoint: H256,
        pub(crate) sig_script: H512,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash, Default)]
    pub struct TransactionOutput {
        pub(crate) value: Value,
        pub(crate) pub_key: H256,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash, Default)]
    pub struct Transaction {
        pub(crate) inputs: Vec<TransactionInput>,
        pub(crate) outputs: Vec<TransactionOutput>,
    }

    #[pallet::storage]
    #[pallet::getter(fn reward_total)]
    pub(super) type RewardTotal<T> = StorageValue<_, Value, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn utxo_hash)]
    pub(super) type UtxoHash<T: Config> =
        StorageMap<_, Blake2_256, H256, TransactionOutput, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        TransactionSuccess(Transaction),
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub genesis_utxos: Vec<TransactionOutput>,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            self.genesis_utxos
                .iter()
                .cloned()
                .for_each(|u| UtxoHash::<T>::insert(BlakeTwo256::hash_of(&u), u));
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}
}
