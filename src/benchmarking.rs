use super::*;

use crate::{Pallet as Utxo, Transaction, TransactionInput, TransactionOutput};
use codec::Encode;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::{EventRecord, RawOrigin};
use sp_core::{sp_std::vec, sr25519::Public, testing::SR25519, H256, H512};
use hex_literal::hex;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    let events = frame_system::Pallet::<T>::events();
    let system_event: <T as frame_system::Config>::Event = generic_event.into();

    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

benchmarks! {
    // only for test
    test_spend {
        // 5Gq2jqhDKtUScUzm9yCJGDDnhYQ8QHuMWiEzzKpjxma9n57R
        let alice_h256 = H256::from([
            210, 191, 75, 132, 77, 254, 253, 103, 114, 168,
            132, 62, 102, 159, 148, 52, 8, 150, 106, 151, 126,
            58, 226, 175, 29, 215, 142, 15, 85, 244, 223, 103
        ]);
        let alice_pub_key = Public::from_h256(alice_h256.clone());

        let genesis_utxo = hex!("931fe49afe365072e71771cd99e13cfb54fa28fad479e23556ff9de6a3dd19a9");
        let genesis_utxo = H256::from(genesis_utxo);

         let mut tx = Transaction {
            inputs: vec![TransactionInput {
                outpoint: genesis_utxo,
                sig_script: H512::zero(),
            }],
            outputs: vec![TransactionOutput::new(50, alice_h256)],
        };

        let alice_sig = frame_support::sp_io::crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();

        tx.inputs[0].sig_script = H512::from(alice_sig);

        let caller: T::AccountId = whitelisted_caller();
    }: spend(RawOrigin::Signed(caller),tx.clone())
    verify {
        assert_last_event::<T>(Event::TransactionSuccess(tx).into());
        assert_eq!(RewardTotal::<T>::get(),50u128);
        assert!(!UtxoStore::<T>::contains_key(genesis_utxo));
    }

    runtime_spend {
        /// ran using mintlayer-node.
        // 0x76584168d10a20084082ed80ec71e2a783abbb8dd6eb9d4893b089228498e9ff
        let alice_h256 = H256::from([
            212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26,
            189, 4, 169, 159, 214,130, 44, 133,88, 133, 76,
            205, 227, 154, 86, 132, 231, 165, 109, 162, 125]
        );
        let alice_pub_key = Public::from_h256(alice_h256.clone());

        let genesis_utxo = H256::from([
            118, 88, 65, 104, 209, 10, 32, 8, 64, 130, 237, 128,
            236, 113, 226, 167, 131, 171, 187, 141, 214, 235, 157,
            72, 147, 176, 137, 34, 132, 152, 233, 255]
        );

        // 0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48
        let bob_h256 = H256::from([
            142, 175, 4, 21, 22, 135, 115, 99, 38, 201, 254, 161,
            126, 37, 252, 82, 135, 97, 54, 147, 201, 18, 144, 156,
            178, 38, 170, 71, 148, 242, 106, 72]
        );
        let bob_pub_key = Public::from_h256(bob_h256.clone());

        // 0x6ceab99702c60b111c12c2867679c5555c00dcd4d6ab40efa01e3a65083bfb6c6f5c1ed3356d7141ec61894153b8ba7fb413bf1e990ed99ff6dee5da1b24fd83
        let alice_sigscript = H512::from([
            108, 234, 185, 151, 2, 198, 11, 17, 28, 18, 194, 134,
            118, 121, 197, 85, 92, 0, 220, 212, 214, 171, 64, 239,
            160, 30, 58, 101, 8, 59, 251, 108, 111, 92, 30, 211, 53,
            109, 113, 65, 236, 97, 137, 65, 83, 184, 186, 127, 180,
            19, 191, 30, 153, 14, 217, 159, 246, 222, 229, 218, 27,
            36, 253, 131]
        );

        let mut tx = Transaction {
            inputs: vec![ TransactionInput {
                outpoint: genesis_utxo.clone(),
                sig_script: H512::zero()
            }],
            outputs: vec![ TransactionOutput::new(50, bob_h256)]
        };

        tx.inputs[0].sig_script = alice_sigscript;

        let caller: T::AccountId = whitelisted_caller();
    }: spend(RawOrigin::Signed(caller), tx.clone())
    verify {
        assert_last_event::<T>(Event::TransactionSuccess(tx).into());
        assert_eq!(RewardTotal::<T>::get(),50u128);
        assert!(!UtxoStore::<T>::contains_key(genesis_utxo));
    }
}

// only for test
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn spend() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_test_spend::<Test>());
        });
    }
}
