use std::path::PathBuf;

use anyhow::Context;
use xshell::Shell;

#[derive(Debug)]
pub struct ContractRepr {
    pub abi: web3::ethabi::Contract,
    pub bytecode: String,
}

impl ContractRepr {
    pub fn from_value(value: serde_json::Value) -> anyhow::Result<Self> {
        let bytecode_field_name: &str = todo!();
        let bytecode = value
            .get(bytecode_field_name)
            .context("no bytecode field")?
            .as_str()
            .context("bytecode is not a string")?
            .to_string();
        let abi: web3::ethabi::Contract = serde_json::from_value(value).context("Invalid ABI")?;
        Ok(Self { bytecode, abi })
    }
}

#[derive(Debug)]
pub struct Contracts<'a> {
    shell: &'a Shell,
    base_folder: PathBuf,
}

impl<'a> Contracts<'a> {
    pub fn new(shell: &'a Shell, base_folder: PathBuf) -> Self {
        Self { shell, base_folder }
    }

    fn load(&self, relative_path: &str) -> anyhow::Result<ContractRepr> {
        let path = self.base_folder.join(relative_path);
        if !self.shell.path_exists(&path) {
            anyhow::bail!("No such path: {:?}", path);
        }
        let file = self.shell.read_file(&path)?;
        let json: serde_json::Value = serde_json::from_str(&file)?;
        ContractRepr::from_value(json)
    }

    pub fn create2_factory(&self) -> anyhow::Result<ContractRepr> {
        todo!()
    }

    pub fn multicall3(&self) -> anyhow::Result<ContractRepr> {
        todo!()
    }

    pub fn verifier(&self) -> anyhow::Result<ContractRepr> {
        todo!()
    }
}
