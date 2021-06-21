use crate as pallet_utxo;
use pallet_utxo::TransactionOutput;

use frame_support::{parameter_types, traits::GenesisBuild};
use sp_core::{sr25519::Public, testing::SR25519, H256};
use sp_io::TestExternalities;
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_std::vec;

// need to manually import this crate since its no include by default
use hex_literal::hex;

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
pub type Block = frame_system::mocking::MockBlock<Test>;

pub const ALICE_PHRASE: &str =
    "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
pub const GENESIS_UTXO: [u8; 32] =
    hex!("79eabcbd5ef6e958c6a7851b36da07691c19bda1835a08f875aa286911800999");

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
        Utxo: pallet_utxo::{Module, Call, Config<T>, Storage, Event<T>},
        Aura: pallet_aura::{Module, Call, Config<T>, Storage},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
    pub const MinimumPeriod: u64 = 1;

    pub const MaximumBlockLength: u32 = 2 * 1024;
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

// required by pallet_aura
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl pallet_aura::Config for Test {
    type AuthorityId = AuraId;
}

impl pallet_utxo::Config for Test {
    type Event = Event;
    type Call = Call;
    type WeightInfo = crate::weights::WeightInfo<Test>;

    fn authorities() -> Vec<H256> {
        Aura::authorities()
            .iter()
            .map(|x| {
                let r: &Public = x.as_ref();
                r.0.into()
            })
            .collect()
    }
}

fn create_pub_key(keystore: &KeyStore, phrase: &str) -> Public {
    SyncCryptoStore::sr25519_generate_new(keystore, SR25519, Some(phrase)).unwrap()
}

pub fn new_test_ext() -> TestExternalities {
    let keystore = KeyStore::new(); // a key storage to store new key pairs during testing
    let alice_pub_key = create_pub_key(&keystore, ALICE_PHRASE);
    println!("alice pub key: {:?}", alice_pub_key.0);
    println!("gensis: {:?}", GENESIS_UTXO);

    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_utxo::GenesisConfig::<Test> {
        genesis_utxos: vec![TransactionOutput {
            value: 100,
            pub_key: H256::from(alice_pub_key),
        }],
        _marker: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = TestExternalities::from(t);
    ext.register_extension(KeystoreExt(std::sync::Arc::new(keystore)));
    ext
}

pub fn new_test_ext_and_keys() -> (TestExternalities, Public, Public) {
    // other random account generated with subkey
    const KARL_PHRASE: &str =
        "monitor exhibit resource stumble subject nut valid furnace obscure misery satoshi assume";

    let keystore = KeyStore::new();
    let alice_pub_key = create_pub_key(&keystore, ALICE_PHRASE);
    let karl_pub_key = create_pub_key(&keystore, KARL_PHRASE);

    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_utxo::GenesisConfig::<Test> {
        genesis_utxos: vec![TransactionOutput {
            value: 100,
            pub_key: H256::from(alice_pub_key),
        }],
        _marker: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = TestExternalities::from(t);
    ext.register_extension(KeystoreExt(std::sync::Arc::new(keystore)));
    (ext, alice_pub_key, karl_pub_key)
}
