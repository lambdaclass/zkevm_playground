use once_cell::sync::Lazy;
use zksync_era_contracts::ContractLanguage;
use zksync_era_types::{Address, ACCOUNT_CODE_STORAGE_ADDRESS, NONCE_HOLDER_ADDRESS, KNOWN_CODES_STORAGE_ADDRESS, IMMUTABLE_SIMULATOR_STORAGE_ADDRESS, CONTRACT_DEPLOYER_ADDRESS, L1_MESSENGER_ADDRESS, MSG_VALUE_SIMULATOR_ADDRESS, L2_ETH_TOKEN_ADDRESS, KECCAK256_PRECOMPILE_ADDRESS, SHA256_PRECOMPILE_ADDRESS, ECRECOVER_PRECOMPILE_ADDRESS, SYSTEM_CONTEXT_ADDRESS, EVENT_WRITER_ADDRESS, BOOTLOADER_UTILITIES_ADDRESS, BYTECODE_COMPRESSOR_ADDRESS, COMPLEX_UPGRADER_ADDRESS, BOOTLOADER_ADDRESS, block::DeployedContract, AccountTreeId};

use crate::utils;

pub const ROOT: &str = env!("CARGO_MANIFEST_DIR");

pub static SYSTEM_CONTRACT_LIST: [(&str, Address, ContractLanguage); 18] = [
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

pub static SYSTEM_CONTRACTS: Lazy<Vec<DeployedContract>> = Lazy::new(|| {
    SYSTEM_CONTRACT_LIST
        .iter()
        .map(|(name, address, contract_lang)| 
            match contract_lang {
                ContractLanguage::Sol => {
                    DeployedContract {
                        account_id: AccountTreeId::new(*address),
                        bytecode: utils::read_contract_bytecode(format!("{ROOT}/contracts/{name}.json")),
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
