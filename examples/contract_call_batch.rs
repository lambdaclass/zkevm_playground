
use ethers_core::abi::Token;
use zkevm_playground::{compiler, utils};
use zksync_era_test_account::TxType;
use zksync_era_types::{U256, Execute};
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
    
    // Call the contract
    let contract = artifact.abi.unwrap();

    // Call get()
    let function = contract.function("get").unwrap();
    let execute = Execute {
        contract_address: deploy_tx.address,
        calldata: function.encode_input(&[]).unwrap(),
        factory_deps: None,
        value: U256::zero(),
    };
    let call_tx = sender.get_l2_tx_for_execute(execute, None);
    vm.push_transaction(call_tx);

    // Call increment()
    let function = contract.function("increment").unwrap();
    let execute = Execute {
        contract_address: deploy_tx.address,
        calldata: function.encode_input(&[Token::Uint(U256::one())]).unwrap(),
        factory_deps: None,
        value: U256::zero(),
    };
    let call_tx = sender.get_l2_tx_for_execute(execute, None);
    vm.push_transaction(call_tx);

    // Call incrementWithRevertPayable()
    let function = contract.function("incrementWithRevertPayable").unwrap();
    let execute = Execute {
        contract_address: deploy_tx.address,
        calldata: function.encode_input(&[Token::Uint(U256::one()), Token::Bool(true)]).unwrap(),
        factory_deps: None,
        value: U256::zero(),
    };
    let call_tx = sender.get_l2_tx_for_execute(execute, None);
    vm.push_transaction(call_tx);

    // Call incrementWithRevert()
    let function = contract.function("incrementWithRevert").unwrap();
    let execute = Execute {
        contract_address: deploy_tx.address,
        calldata: function.encode_input(&[Token::Uint(U256::one()), Token::Bool(true)]).unwrap(),
        factory_deps: None,
        value: U256::zero(),
    };
    let call_tx = sender.get_l2_tx_for_execute(execute, None);
    vm.push_transaction(call_tx);

    // Execute the batch
    let batch_execution_result = vm.execute(VmExecutionMode::Batch).result;
    log::info!("{batch_execution_result:?}");
}
