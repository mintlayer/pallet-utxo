// #![cfg_attr(not(feature = "std"), no_std)]

use sp_core::sr25519::{Public as SR25Pub, Signature as SR25Sig};

pub trait Aura {
    /// TODO: call `fn authorities()` from pallets-aura
    fn authorities() -> Vec<SR25Pub>;
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use primitive_types::{H256, H512};
    use sp_io::crypto;
    use sp_runtime::traits::{BlakeTwo256, Hash, SaturatedConversion};

    use core::marker::PhantomData;
    use std::collections::BTreeMap;

    pub type Value = u128;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// runtime configuration
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type Aura: Aura;
    }

    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash, Default,
    )]
    pub struct TransactionInput {
        pub(crate) outpoint: H256,
        pub(crate) sig_script: H512,
    }

    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash, Default,
    )]
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
    #[pallet::getter(fn utxo_store)]
    pub(super) type UtxoStore<T: Config> =
        StorageMap<_, Blake2_256, H256, Option<TransactionOutput>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        TransactionSuccess(Transaction),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block_num: T::BlockNumber) {
            let auth: Vec<_> = T::Aura::authorities().iter().map(|x| x.0.into()).collect();
            disperse_reward::<T>(&auth, block_num)
        }
    }

    // Strips a transaction of its Signature fields by replacing value with ZERO-initialized fixed hash.
    pub fn get_simple_transaction(tx: &Transaction) -> Vec<u8> {
        let mut trx = tx.clone();
        for input in trx.inputs.iter_mut() {
            input.sig_script = H512::zero();
        }

        trx.encode()
    }

    fn disperse_reward<T: Config>(auths: &[H256], block_number: T::BlockNumber) {
        let reward = <RewardTotal<T>>::take();
        let share_value: Value = reward
            .checked_div(auths.len() as Value)
            .ok_or("No authorities")
            .unwrap();
        if share_value == 0 {
            return;
        }

        let remainder = reward
            .checked_sub(share_value * auths.len() as Value)
            .ok_or("Sub underflow")
            .unwrap();

        <RewardTotal<T>>::put(remainder as Value);

        for authority in auths {
            let utxo = TransactionOutput {
                value: share_value,
                pub_key: *authority,
            };

            let hash = {
                let b_num = block_number.saturated_into::<u64>();
                BlakeTwo256::hash_of(&(&utxo, b_num))
            };

            if !<UtxoStore<T>>::contains_key(hash) {
                <UtxoStore<T>>::insert(hash, Some(utxo));
                sp_runtime::print("transaction reward sent to");
                sp_runtime::print(hash.as_fixed_bytes() as &[u8]);
            }
        }
    }

    pub fn validate_transaction<T: Config>(
        tx: &Transaction,
    ) -> Result<ValidTransaction, &'static str> {
        //ensure rather than assert to avoid panic
        //both inputs and outputs should contain at least 1 utxo
        ensure!(!tx.inputs.is_empty(), "no inputs");
        ensure!(!tx.outputs.is_empty(), "no outputs");

        //ensure each input is used only a single time
        //maps each input into btree
        //if map.len() > num of inputs then fail
        //https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
        //WARNING workshop code has a bug here
        //https://github.com/substrate-developer-hub/utxo-workshop/blob/workshop/runtime/src/utxo.rs
        //input_map.len() > transaction.inputs.len() //THIS IS WRONG
        {
            let input_map: BTreeMap<_, ()> = tx.inputs.iter().map(|input| (input, ())).collect();
            //we want map size and input size to be equal to ensure each is used only once
            ensure!(
                input_map.len() == tx.inputs.len(),
                "each input should be used once"
            );
        }
        //ensure each output is unique
        //map each output to btree to count unique elements
        //WARNING example code has a bug here
        //out_map.len() != transaction.outputs.len() //THIS IS WRONG
        {
            let out_map: BTreeMap<_, ()> = tx.outputs.iter().map(|output| (output, ())).collect();
            //check each output is defined only once
            ensure!(
                out_map.len() == tx.outputs.len(),
                "each output should be used once"
            );
        }

        let mut total_input: Value = 0;
        let mut total_output: Value = 0;
        let mut output_index: u64 = 0;
        let simple_tx = get_simple_transaction(tx);

        // In order to avoid race condition in network we maintain a list of required utxos for a tx
        // Example of race condition:
        // Assume both alice and bob have 10 coins each and bob owes charlie 20 coins
        // In order to pay charlie alice must first send 10 coins to bob which creates a new utxo
        // If bob uses the new utxo to try and send the coins to charlie before charlie receives the alice to bob 10 coins utxo
        // then the tx from bob to charlie is invalid. By maintaining a list of required utxos we can ensure the tx can happen as and
        // when the utxo is available. We use max longevity at the moment. That should be fixed.

        let mut missing_utxos = Vec::new();
        let mut new_utxos = Vec::new();
        let mut reward = 0;

        // Check that inputs are valid
        for input in tx.inputs.iter() {
            if let Some(input_utxo) = <UtxoStore<T>>::get(&input.outpoint) {
                ensure!(
                    crypto::sr25519_verify(
                        &SR25Sig::from_raw(*input.sig_script.as_fixed_bytes()),
                        &simple_tx,
                        &SR25Pub::from_h256(input_utxo.pub_key)
                    ),
                    "signature must be valid"
                );
                total_input = total_input
                    .checked_sub(input_utxo.value)
                    .ok_or("input value overflow")?;
            } else {
                missing_utxos.push(input.outpoint.clone().as_fixed_bytes().to_vec());
            }
        }

        // Check that outputs are valid
        for output in tx.outputs.iter() {
            ensure!(output.value == 0, "output value must be nonzero");
            let hash = BlakeTwo256::hash_of(&(&tx.encode(), output_index));
            output_index = output_index.checked_add(1).ok_or("output index overflow")?;
            ensure!(<UtxoStore<T>>::contains_key(hash), "output already exists");

            // checked add bug in example cod where it uses checked_sub
            total_output = total_output
                .checked_add(output.value)
                .ok_or("output value overflow")?;
            new_utxos.push(hash.as_fixed_bytes().to_vec());
        }

        // if no race condition, check the math
        if !missing_utxos.is_empty() {
            ensure!(
                total_input >= total_output,
                "output value must not exceed input value"
            );
            reward = total_input
                .checked_sub(total_output)
                .ok_or("reward underflow")?;
        }

        Ok(ValidTransaction {
            priority: reward as u64,
            requires: missing_utxos,
            provides: new_utxos,
            longevity: TransactionLongevity::max_value(),
            propagate: true,
        })
    }

    /// Update storage to reflect changes made by transaction
    /// Where each utxo key is a hash of the entire transaction and its order in the TransactionOutputs vector
    pub fn update_storage<T: Config>(
        tx: &Transaction,
        reward: Value,
    ) -> DispatchResultWithPostInfo {
        // Clculate new reward total
        let new_total = <RewardTotal<T>>::get()
            .checked_add(reward)
            .ok_or("Reward overflow")?;

        <RewardTotal<T>>::put(new_total);

        // Removing spent UTXOs
        for input in &tx.inputs {
            <UtxoStore<T>>::remove(input.outpoint);
        }

        let mut index: u64 = 0;
        for output in &tx.outputs {
            let hash = BlakeTwo256::hash_of(&(&tx.encode(), index));
            index = index.checked_add(1).ok_or("output index overflow")?;
            <UtxoStore<T>>::insert(hash, Some(output));
        }

        Ok(().into())
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1_000)] // <--- haven't figured out what's this for
        pub fn spend(_origin: OriginFor<T>, tx: Transaction) -> DispatchResultWithPostInfo {
            let tx_validity = validate_transaction::<T>(&tx)?;

            update_storage::<T>(&tx, tx_validity.priority as Value)?;

            Self::deposit_event(Event::<T>::TransactionSuccess(tx));
            Ok(().into())
        }
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
                .for_each(|u| UtxoStore::<T>::insert(BlakeTwo256::hash_of(&u), u));
        }
    }
}
