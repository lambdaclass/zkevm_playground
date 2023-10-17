use ethers_core::abi::Token;
use zkevm_playground::{compiler, utils};
use zksync_era_test_account::TxType;
use zksync_era_types::U256;
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
    let deployment_execution_result = vm.execute(VmExecutionMode::OneTx).result;
    log::info!("{deployment_execution_result:?}");
    
    // Call the contract
    let contract = artifact.abi.unwrap();
    let contract_address = deploy_tx.address;

    let function_name = "get";
    let get_call_tx = utils::build_call_tx(&mut sender, &contract, &contract_address, function_name, &[]);
    vm.push_transaction(get_call_tx);
    let get_call_execution_result = vm.execute(VmExecutionMode::OneTx).result;
    log::info!("{get_call_execution_result:?}");

    let function_name = "increment";
    let function_args = [Token::Uint(U256::one())];
    let increment_call_tx = utils::build_call_tx(&mut sender, &contract, &contract_address, function_name, &function_args);
    vm.push_transaction(increment_call_tx);
    let increment_call_execution_result = vm.execute(VmExecutionMode::OneTx).result;
    log::info!("{increment_call_execution_result:?}");


    let function_name = "incrementWithRevertPayable";
    let function_args = [Token::Uint(U256::one()), Token::Bool(true)];
    let increment_call_tx = utils::build_call_tx(&mut sender, &contract, &contract_address, function_name, &function_args);
    vm.push_transaction(increment_call_tx);
    let increment_call_execution_result = vm.execute(VmExecutionMode::OneTx).result;
    log::info!("{increment_call_execution_result:?}");

    let function_name = "incrementWithRevert";
    let function_args = [Token::Uint(U256::one()), Token::Bool(true)];
    let increment_call_tx = utils::build_call_tx(&mut sender, &contract, &contract_address, function_name, &function_args);
    vm.push_transaction(increment_call_tx);
    let increment_call_execution_result = vm.execute(VmExecutionMode::OneTx).result;
    log::info!("{increment_call_execution_result:?}");
}
