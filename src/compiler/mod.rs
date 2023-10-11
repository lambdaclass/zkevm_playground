use self::{project::ZKSProject, output::ZKSArtifact};
use std::{path::PathBuf, str::FromStr};
use ethers_solc::{ProjectPathsConfig, Project, info::ContractInfo};

pub mod errors;
mod output;
pub mod project;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

pub fn compile(contract_path: &str, contract_name: &str) -> ZKSArtifact {
    let mut root = PathBuf::from(ROOT);
    root.push::<PathBuf>(contract_path.clone().into());
    let zk_project = ZKSProject::from(
        Project::builder()
            .paths(ProjectPathsConfig::builder().build_with_root(root))
            .set_auto_detect(true)
            .build()
            .unwrap(),
    );
    let compilation_output = zk_project.compile().unwrap();
    compilation_output
        .find_contract(ContractInfo::from_str(&format!(
            "{contract_path}:{contract_name}"
        )).unwrap()).unwrap().clone()
}
