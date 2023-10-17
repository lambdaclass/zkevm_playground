use zksync_era_contracts::{BaseSystemContracts, SystemContractCode};
use zksync_era_state::{InMemoryStorage, StorageView, WriteStorage};
use zksync_era_test_account::{Account, TxType, DeployContractsTx};
use zksync_era_types::{get_code_key, get_is_account_key, L1BatchNumber, helpers::unix_timestamp_ms, Address, block::legacy_miniblock_hash, MiniblockNumber, ProtocolVersionId, L2ChainId, ethabi::{Token, Contract, self}, Execute, CONTRACT_DEPLOYER_ADDRESS, U256, utils::{deployed_address_create, storage_key_for_eth_balance}, Nonce, Transaction};
use zksync_era_utils::{bytecode::hash_bytecode, u256_to_h256, bytes_to_be_words};
use zksync_era_vm::{Vm, L1BatchEnv, L2BlockEnv, SystemEnv, constants::BLOCK_GAS_LIMIT, TxExecutionMode, HistoryEnabled};

use serde_json::Value;
use std::{fs::File, str::FromStr, cell::RefCell, rc::Rc};

use crate::constants;

pub fn default_l1_batch(number: L1BatchNumber) -> L1BatchEnv {
    let timestamp = unix_timestamp_ms();
    L1BatchEnv {
        previous_batch_hash: None,
        number,
        timestamp,
        l1_gas_price: 50_000_000_000,   // 50 gwei
        fair_l2_gas_price: 250_000_000, // 0.25 gwei
        fee_account: Address::random(),
        enforced_base_fee: None,
        first_l2_block: L2BlockEnv {
            number: 1,
            timestamp,
            prev_block_hash: legacy_miniblock_hash(MiniblockNumber(0)),
            max_virtual_blocks_to_create: 100,
        },
    }
}

pub fn default_system_env() -> SystemEnv {
    let base_system_smart_contracts = {
        let playground_bytecode = read_precompile_bytecode(format!("{}/contracts/playground_batch.yul.zbin", constants::ROOT));
        let hash = hash_bytecode(&playground_bytecode);
    
        let bootloader = SystemContractCode {
            code: bytes_to_be_words(playground_bytecode),
            hash,
        };
    
        let bytecode = read_contract_bytecode(format!("{}/contracts/DefaultAccount.json", constants::ROOT));
        let hash = hash_bytecode(&bytecode);
    
        let default_aa = SystemContractCode {
            code: bytes_to_be_words(bytecode),
            hash,
        };
        
        BaseSystemContracts { bootloader, default_aa}
    };

    SystemEnv {
        zk_porter_available: false,
        version: ProtocolVersionId::latest(),
        base_system_smart_contracts,
        gas_limit: BLOCK_GAS_LIMIT,
        execution_mode: TxExecutionMode::VerifyExecute,
        default_validation_computational_gas_limit: BLOCK_GAS_LIMIT,
        chain_id: L2ChainId::from_str("270").unwrap(),
    }
}

pub fn default_empty_storage(contracts: &[ContractToDeploy]) -> Rc<RefCell<StorageView<InMemoryStorage>>> {
    let mut raw_storage = InMemoryStorage::with_custom_system_contracts_and_chain_id(L2ChainId::from_str("270").unwrap(), hash_bytecode, constants::SYSTEM_CONTRACTS.clone());
    insert_contracts(&mut raw_storage, contracts);
    StorageView::new(raw_storage).to_rc_ptr()
}

pub fn read_precompile_bytecode(path: String) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

pub fn read_contract_bytecode(path: String) -> Vec<u8> {
    let artifact = serde_json::from_reader::<_, Value>(File::open(path).unwrap()).unwrap();
    let bytecode = artifact["bytecode"].as_str().unwrap().strip_prefix("0x").unwrap();
    hex::decode(bytecode).unwrap()
}

// (bytecode, address, is_account)
type ContractToDeploy = (Vec<u8>, Address, bool);

// Inserts the contracts into the test environment, bypassing the
// deployer system contract. Besides the reference to storage
// it accepts a `contracts` tuple of information about the contract
// and whether or not it is an account.
pub fn insert_contracts(raw_storage: &mut InMemoryStorage, contracts: &[ContractToDeploy]) {
    for (contract, address, is_account) in contracts {
        let deployer_code_key = get_code_key(address);
        raw_storage.set_value(deployer_code_key, hash_bytecode(contract));

        if *is_account {
            let is_account_key = get_is_account_key(address);
            raw_storage.set_value(is_account_key, u256_to_h256(1_u32.into()));
        }

        raw_storage.store_factory_dep(hash_bytecode(contract), contract.to_vec());
    }
}

pub fn random_rich_account(storage: Rc<RefCell<StorageView<InMemoryStorage>>>) -> Account {
    let account = Account::random();
    let key = storage_key_for_eth_balance(&account.address);
    storage
        .as_ref()
        .borrow_mut()
        .set_value(key, u256_to_h256(U256::from(10u64.pow(19))));
    account
}

pub fn build_deploy_tx(sender: &mut Account, code: &[u8], calldata: Option<&[Token]>, mut factory_deps: Vec<Vec<u8>>, tx_type: TxType) -> DeployContractsTx {
    let deployer = serde_json::from_value::<Contract>(serde_json::from_reader::<_, Value>(File::open(format!("{}/contracts/ContractDeployer.json", constants::ROOT)).unwrap()).unwrap()["abi"].take()).unwrap();

    let contract_function = deployer.function("create").unwrap();

    let calldata = calldata.map(ethabi::encode);
    let code_hash = hash_bytecode(code);
    let params = [
        Token::FixedBytes(vec![0u8; 32]),
        Token::FixedBytes(code_hash.0.to_vec()),
        Token::Bytes(calldata.unwrap_or_default().to_vec()),
    ];
    factory_deps.push(code.to_vec());
    let calldata = contract_function
        .encode_input(&params)
        .expect("failed to encode parameters");

    let execute = Execute {
        contract_address: CONTRACT_DEPLOYER_ADDRESS,
        calldata,
        factory_deps: Some(factory_deps),
        value: U256::zero(),
    };

    let tx = match tx_type {
        TxType::L2 => sender.get_l2_tx_for_execute(execute, None),
        TxType::L1 { serial_id } => sender.get_l1_tx(execute, serial_id),
    };

    // For L1Tx we usually use nonce 0
    let address = deployed_address_create(sender.address, (tx.nonce().unwrap_or(Nonce(0)).0).into());
    DeployContractsTx {
        tx,
        bytecode_hash: code_hash,
        address,
    }
}

pub fn build_call_tx(sender: &mut Account, contract: &Contract, contract_address: &Address, function_name: &str, function_args: &[Token]) -> Transaction {
    let function = contract.function(function_name).unwrap();
    let execute = Execute {
        contract_address: contract_address.clone(),
        calldata: function.encode_input(function_args).unwrap(),
        factory_deps: None,
        value: U256::zero(),
    };
    sender.get_l2_tx_for_execute(execute, None)
}

pub fn default_vm(storage: Rc<RefCell<StorageView<InMemoryStorage>>>) -> Vm<StorageView<InMemoryStorage>, HistoryEnabled> {
    let batch_env = default_l1_batch(L1BatchNumber(1));
    let system_env = default_system_env();
    Vm::new(batch_env, system_env, storage, HistoryEnabled)
}
