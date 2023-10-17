use zksync_era_contracts::{BaseSystemContracts, SystemContractCode, ContractLanguage};
use zksync_era_state::{InMemoryStorage, StorageView, WriteStorage};
use zksync_era_test_account::{Account, TxType, DeployContractsTx};
use zksync_era_types::{get_code_key, get_is_account_key, L1BatchNumber, helpers::unix_timestamp_ms, Address, block::{legacy_miniblock_hash, DeployedContract}, MiniblockNumber, ProtocolVersionId, L2ChainId, ethabi::{Token, Contract, self}, Execute, CONTRACT_DEPLOYER_ADDRESS, U256, utils::{deployed_address_create, storage_key_for_eth_balance}, Nonce, ACCOUNT_CODE_STORAGE_ADDRESS, NONCE_HOLDER_ADDRESS, KNOWN_CODES_STORAGE_ADDRESS, IMMUTABLE_SIMULATOR_STORAGE_ADDRESS, L1_MESSENGER_ADDRESS, MSG_VALUE_SIMULATOR_ADDRESS, L2_ETH_TOKEN_ADDRESS, KECCAK256_PRECOMPILE_ADDRESS, SHA256_PRECOMPILE_ADDRESS, ECRECOVER_PRECOMPILE_ADDRESS, SYSTEM_CONTEXT_ADDRESS, EVENT_WRITER_ADDRESS, BOOTLOADER_UTILITIES_ADDRESS, BYTECODE_COMPRESSOR_ADDRESS, COMPLEX_UPGRADER_ADDRESS, BOOTLOADER_ADDRESS, AccountTreeId, Transaction};
use zksync_era_utils::{bytecode::hash_bytecode, u256_to_h256, bytes_to_be_words};
use zksync_era_vm::{Vm, L1BatchEnv, L2BlockEnv, SystemEnv, constants::BLOCK_GAS_LIMIT, TxExecutionMode, HistoryEnabled, VmExecutionMode};

use serde_json::Value;
use once_cell::sync::Lazy;
use std::{fs::File, str::FromStr, cell::RefCell, rc::Rc};

mod compiler;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

static SYSTEM_CONTRACT_LIST: [(&str, Address, ContractLanguage); 18] = [
    (
        "AccountCodeStorage",
        ACCOUNT_CODE_STORAGE_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "NonceHolder",
        NONCE_HOLDER_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "KnownCodesStorage",
        KNOWN_CODES_STORAGE_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "ImmutableSimulator",
        IMMUTABLE_SIMULATOR_STORAGE_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "ContractDeployer",
        CONTRACT_DEPLOYER_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "L1Messenger",
        L1_MESSENGER_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "MsgValueSimulator",
        MSG_VALUE_SIMULATOR_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "L2EthToken",
        L2_ETH_TOKEN_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "Keccak256",
        KECCAK256_PRECOMPILE_ADDRESS,
        ContractLanguage::Yul,
    ),
    (
        "SHA256",
        SHA256_PRECOMPILE_ADDRESS,
        ContractLanguage::Yul,
    ),
    (
        "Ecrecover",
        ECRECOVER_PRECOMPILE_ADDRESS,
        ContractLanguage::Yul,
    ),
    (
        "SystemContext",
        SYSTEM_CONTEXT_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "EventWriter",
        EVENT_WRITER_ADDRESS,
        ContractLanguage::Yul,
    ),
    (
        "BootloaderUtilities",
        BOOTLOADER_UTILITIES_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "BytecodeCompressor",
        BYTECODE_COMPRESSOR_ADDRESS,
        ContractLanguage::Sol,
    ),
    (
        "ComplexUpgrader",
        COMPLEX_UPGRADER_ADDRESS,
        ContractLanguage::Sol,
    ),
    // For now, only zero address and the bootloader address have empty bytecode at the init
    // In the future, we might want to set all of the system contracts this way.
    ("EmptyContract", Address::zero(), ContractLanguage::Sol),
    (
        "EmptyContract",
        BOOTLOADER_ADDRESS,
        ContractLanguage::Sol,
    ),
];

static SYSTEM_CONTRACTS: Lazy<Vec<DeployedContract>> = Lazy::new(|| {
    SYSTEM_CONTRACT_LIST
        .iter()
        .map(|(name, address, contract_lang)| 
            match contract_lang {
                ContractLanguage::Sol => {
                    DeployedContract {
                        account_id: AccountTreeId::new(*address),
                        bytecode: read_contract_bytecode(format!("{ROOT}/contracts/{name}.json")),
                    }
                },
                ContractLanguage::Yul => {
                    DeployedContract {
                        account_id: AccountTreeId::new(*address),
                        bytecode: std::fs::read(format!("{ROOT}/contracts/{name}.yul.zbin")).unwrap()
                    }
                },
            }
        )
        .collect::<Vec<_>>()
});

fn default_l1_batch(number: L1BatchNumber) -> L1BatchEnv {
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

fn default_system_env() -> SystemEnv {
    let base_system_smart_contracts = {
        let playground_bytecode = read_precompile_bytecode(format!("{ROOT}/contracts/playground_batch.yul.zbin"));
        let hash = hash_bytecode(&playground_bytecode);
    
        let bootloader = SystemContractCode {
            code: bytes_to_be_words(playground_bytecode),
            hash,
        };
    
        let bytecode = read_contract_bytecode(format!("{ROOT}/contracts/DefaultAccount.json"));
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

fn default_empty_storage(contracts: &[ContractToDeploy]) -> Rc<RefCell<StorageView<InMemoryStorage>>> {
    let mut raw_storage = InMemoryStorage::with_custom_system_contracts_and_chain_id(L2ChainId::from_str("270").unwrap(), hash_bytecode, SYSTEM_CONTRACTS.clone());
    insert_contracts(&mut raw_storage, contracts);
    StorageView::new(raw_storage).to_rc_ptr()
}

fn read_precompile_bytecode(path: String) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

fn read_contract_bytecode(path: String) -> Vec<u8> {
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
fn insert_contracts(raw_storage: &mut InMemoryStorage, contracts: &[ContractToDeploy]) {
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

fn random_rich_account(storage: Rc<RefCell<StorageView<InMemoryStorage>>>) -> Account {
    let account = Account::random();
    let key = storage_key_for_eth_balance(&account.address);
    storage
        .as_ref()
        .borrow_mut()
        .set_value(key, u256_to_h256(U256::from(10u64.pow(19))));
    account
}

fn build_deploy_tx(sender: &mut Account, code: &[u8], calldata: Option<&[Token]>, mut factory_deps: Vec<Vec<u8>>, tx_type: TxType) -> DeployContractsTx {
    let deployer = serde_json::from_value::<Contract>(serde_json::from_reader::<_, Value>(File::open(format!("{ROOT}/contracts/ContractDeployer.json")).unwrap()).unwrap()["abi"].take()).unwrap();

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

fn build_call_tx(sender: &mut Account, contract: &Contract, contract_address: &Address, function_name: &str, function_args: &[Token]) -> Transaction {
    let function = contract.function(function_name).unwrap();
    let execute = Execute {
        contract_address: contract_address.clone(),
        calldata: function.encode_input(function_args).unwrap(),
        factory_deps: None,
        value: U256::zero(),
    };
    sender.get_l2_tx_for_execute(execute, None)
}

fn default_vm(storage: Rc<RefCell<StorageView<InMemoryStorage>>>) -> Vm<StorageView<InMemoryStorage>, HistoryEnabled> {
    let batch_env = default_l1_batch(L1BatchNumber(1));
    let system_env = default_system_env();
    Vm::new(batch_env, system_env, storage, HistoryEnabled)
}

fn main() {
    env_logger::builder()
        // .filter_module("reqwest::connect", log::LevelFilter::Off)
        .filter_level(log::LevelFilter::Debug)
        .init();

    let storage = default_empty_storage(&[]);
    let artifact = compiler::compile("test_contracts/counter/src/Counter.sol", "Counter");
    let mut sender = random_rich_account(storage.clone());
    
    // Deploy the contract
    let deploy_tx = build_deploy_tx(&mut sender, &artifact.bin.unwrap(), None, vec![], TxType::L2);
    let mut vm = default_vm(storage);
    vm.push_transaction(deploy_tx.tx.clone());
    let deployment_execution_result = vm.execute(VmExecutionMode::OneTx).result;
    log::info!("{deployment_execution_result:?}");
    
    // Call the contract
    let contract = artifact.abi.unwrap();
    let contract_address = deploy_tx.address;

    let function_name = "get";
    let get_call_tx = build_call_tx(&mut sender, &contract, &contract_address, function_name, &[]);
    vm.push_transaction(get_call_tx);
    let get_call_execution_result = vm.execute(VmExecutionMode::OneTx).result;
    log::info!("{get_call_execution_result:?}");

    let function_name = "increment";
    let function_args = [Token::Uint(U256::one())];
    let increment_call_tx = build_call_tx(&mut sender, &contract, &contract_address, function_name, &function_args);
    vm.push_transaction(increment_call_tx);
    let increment_call_execution_result = vm.execute(VmExecutionMode::OneTx).result;
    log::info!("{increment_call_execution_result:?}");


    let function_name = "incrementWithRevertPayable";
    let function_args = [Token::Uint(U256::one()), Token::Bool(true)];
    let increment_call_tx = build_call_tx(&mut sender, &contract, &contract_address, function_name, &function_args);
    vm.push_transaction(increment_call_tx);
    let increment_call_execution_result = vm.execute(VmExecutionMode::OneTx).result;
    log::info!("{increment_call_execution_result:?}");

    let function_name = "incrementWithRevert";
    let function_args = [Token::Uint(U256::one()), Token::Bool(true)];
    let increment_call_tx = build_call_tx(&mut sender, &contract, &contract_address, function_name, &function_args);
    vm.push_transaction(increment_call_tx);
    let increment_call_execution_result = vm.execute(VmExecutionMode::OneTx).result;
    log::info!("{increment_call_execution_result:?}");
}
