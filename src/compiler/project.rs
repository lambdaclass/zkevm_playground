use ethers_solc::{utils::source_files, Project};

use super::{errors::ZKCompilerError, output::ZKSCompilationOutput};
use std::path::PathBuf;

pub struct ZKSProject {
    pub base_project: Project,
}

impl From<Project> for ZKSProject {
    fn from(base_project: Project) -> Self {
        Self { base_project }
    }
}

impl ZKSProject {
    pub fn compile(&self) -> Result<ZKSCompilationOutput, ZKCompilerError> {
        let zksolc_path = PathBuf::from("src/compiler/bin/zksolc");
        let solc_path = PathBuf::from("src/compiler/bin/solc-macos");

        let command = &mut std::process::Command::new(zksolc_path);
        command
            .arg("--solc")
            .arg(solc_path)
            .arg("--combined-json")
            .arg("abi,bin")
            .arg("--")
            .args(source_files(self.base_project.root()));

        let command_output = command.output().map_err(|e| {
            ZKCompilerError::CompilationError(format!("failed to execute zksolc: {e}"))
        })?;

        let compilation_output = String::from_utf8_lossy(&command_output.stdout)
            .into_owned()
            .trim()
            .to_owned();

        serde_json::from_str(&compilation_output)
            .map_err(|e| ZKCompilerError::CompilationError(e.to_string()))
    }
}
