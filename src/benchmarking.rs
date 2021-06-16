use super::*;

use crate::{Pallet as Utxo, Transaction, TransactionInput, TransactionOutput};
use codec::Encode;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::{EventRecord, RawOrigin};
use primitive_types::{H256, H512};
use sp_core::{sp_std::str::FromStr, sr25519::Public, testing::SR25519};
use sp_runtime::traits::{BlakeTwo256, Hash};

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    let events = frame_system::Module::<T>::events();
    let system_event: <T as frame_system::Config>::Event = generic_event.into();

    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

fn alice() -> (Public, Transaction) {
    let alice_pub_key =
        Public::from_str("5Gq2jqhDKtUScUzm9yCJGDDnhYQ8QHuMWiEzzKpjxma9n57R").unwrap();
    let alice_h256 = H256::from(alice_pub_key.clone());
    let genesis_utxo =
        H256::from_str("0x79eabcbd5ef6e958c6a7851b36da07691c19bda1835a08f875aa286911800999")
            .unwrap();

    let mut tx = Transaction {
        inputs: vec![TransactionInput {
            outpoint: genesis_utxo,
            sig_script: H512::zero(),
        }],
        outputs: vec![TransactionOutput {
            value: 50,
            pub_key: alice_h256,
        }],
    };

    let alice_sig = sp_io::crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();

    tx.inputs[0].sig_script = H512::from(alice_sig);

    (alice_pub_key, tx)
}

benchmarks! {
    spend {
        let s in 0 .. 100; //TODO

        let (alice_pub_key,tx) = alice();

        let caller: T::AccountId = whitelisted_caller(); //TODO

    }: _(RawOrigin::Signed(caller),tx.clone())
    verify {
        assert_last_event::<T>(Event::TransactionSuccess(tx).into())
        //TODO: assert_eq!(RewardTotal::<T>::get(),s as u128);
    }
}

impl_benchmark_test_suite!(Utxo, crate::mock::new_test_ext(), crate::mock::Test);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn spend() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_spend::<Test>());
        });
    }

    //TODO add more tests
}
