
use zkevm_playground::{compiler, utils};
use zksync_era_test_account::TxType;
use zksync_era_vm::VmExecutionMode;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let storage = utils::default_empty_storage(&[]);
    let artifact = compiler::compile("test_contracts/counter/src/Counter.sol", "Counter");
    let mut sender = utils::random_rich_account(storage.clone());
    
    // Deploy the contract
    let deploy_tx = utils::build_deploy_tx(&mut sender, &artifact.bin.unwrap(), None, vec![], TxType::L2);
    let mut vm = utils::default_vm(storage);
    vm.push_transaction(deploy_tx.tx.clone());
    vm.execute(VmExecutionMode::Batch);
}
