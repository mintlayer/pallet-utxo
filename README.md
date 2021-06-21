# pallet-utxo
Utxo support, based on [Substrate's workshop](https://github.com/substrate-developer-hub/utxo-workshop).

This is only the pallet; no _node_ and _runtime_ implementation.

<span style="color:red">**NOTE!!!** The test cases will not compile for this branch.</span> 

### How to run the benchmark in [mintlayer-node](https://github.com/mintlayer/mintlayer-node):
1. Insert this pallet-utxo crate in [pallets directory](https://github.com/mintlayer/mintlayer-node/tree/master/pallets).  

2. At runtime's [Cargo.toml](https://github.com/mintlayer/mintlayer-node/blob/master/runtime/Cargo.toml):  
  2.1. add to local dependencies:`pallet-utxo = {default-features = false, path = "../pallets/utxo" }`  
   2.2. add to std features: `'pallet-utxo/std`
   
3. At runtime's [lib.rs](https://github.com/mintlayer/mintlayer-node/blob/master/runtime/src/lib.rs):  
3.1. Import the following:
   ```rust
   pub use pallet_utxo;
   use sp_runtime::transaction_validity::{TransactionValidityError, InvalidTransaction};
   use pallet_utxo::WeightInfo;
   ```
   3.2. Add the utxo config:
    ```rust
    impl pallet_utxo::Config for Runtime {
        type Event = Event;
        type Call = Call;
        type WeightInfo = pallet_utxo::weights::WeightInfo<Runtime>;
    
    
        fn authorities() -> Vec<H256> {
            Aura::authorities()
                .iter()
                .map(|x| {
                    let r: &sp_core::sr25519::Public = x.as_ref();
                    r.0.into()
                })
                .collect()
       }
   }
    ```
   3.3. Add into `construct_runtime!` this line: `Utxo: pallet_utxo::{Pallet, Call, Config<T>, Storage, Event<T>},`  
3.4. inside the function `validate_transaction()`, add the ff. lines:
   ```rust
   if let Some(pallet_utxo::Call::spend(ref tx)) = 
        IsSubType::<pallet_utxo::Call::<Runtime>>::is_sub_type(&tx.function) {
            match pallet_utxo::validate_transaction::<Runtime>(&tx) {
                Ok(valid_tx) => { return Ok(valid_tx); }
                Err(e) => {
                    sp_runtime::print(e);
                    return Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(1)));
                }
            }
        }
   ```
   3.5. In the function `fn dispatch_benchmark()`, add another line: `add_benchmark!(params, batches, pallet_utxo, Utxo);`  
4. In node's [chain_spec.rs](https://github.com/mintlayer/mintlayer-node/blob/master/node/src/chain_spec.rs):  
4.1. Import the ff:
   ```rust 
   use node_template_runtime::UtxoConfig;
   use sp_runtime::traits::{BlakeTwo256, Hash};
   use sp_core:H256;
   ```
   4.2. add one more param on function `testnet_genesis()`: `endowed_utxos: Vec<sr25519::Public>`  
4.3. inside function `testnet_genesis()`, create the genesis utxo:
    ```rust
    let genesis:Vec<pallet_utxo::TransactionOutput> = endowed_utxos.iter().map(|x| {
        let pub_key = H256::from_slice(x.as_slice());
        println!("The public key: {:?}", x.0);
        println!("The h256 pub key: {:?}", pub_key);
        let tx_output = pallet_utxo::TransactionOutput::new(
            100 as pallet_utxo::Value,
            pub_key
        );
    
        let blake_hash = BlakeTwo256::hash_of(&tx_output);
        println!("blake hash: {:?}", blake_hash.0);
      
        tx_output
    }).collect();
    ```
   4.4. Still inside `testnet_genesis()` function, add to the `GenesisConfig`:
    ```rust
    pallet_utxo: UtxoConfig {
                genesis_utxos: genesis,
                _marker: Default::default()
            }
    ```  
5. On the terminal, move to the node directory and run ` cargo b --release --features runtime-benchmarks`
6. Go back to the workspace directory `$> cd ..` and run: 
```bash
 RUST_LOG=runtime=debug 
 target/release/node-template benchmark 
 --chain dev 
 --execution=wasm 
 --wasm-execution=compiled 
 --pallet pallet_utxo 
 --extrinsic runtime_spend 
 --steps 20 
 --repeat 10 
 --output . 
 --raw
```

   