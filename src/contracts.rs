use std::path::PathBuf;

use anyhow::Context;
use xshell::Shell;

const COMMON_PREFIX_L1: &str = ".contracts/l1-contracts/artifacts/cache/solpp-generated-contracts/";

#[derive(Debug)]
pub struct ContractRepr {
    pub abi: web3::ethabi::Contract,
    pub bytecode: String,
}

impl ContractRepr {
    pub fn new(file: String) -> anyhow::Result<Self> {
        let value: serde_json::Value = serde_json::from_str(&file)?;
        let bytecode = value
            .get("bytecode")
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

    fn load_l1(&self, relative_path: &str) -> anyhow::Result<ContractRepr> {
        let path = self.base_folder.join(COMMON_PREFIX_L1).join(relative_path);
        if !self.shell.path_exists(&path) {
            anyhow::bail!("No such path: {:?}", path);
        }
        let file = self.shell.read_file(&path)?;
        ContractRepr::new(file)
    }

    pub fn create2_factory(&self) -> anyhow::Result<ContractRepr> {
        self.load_l1("dev-contracts/SingletonFactory.sol/SingletonFactory.json")
    }

    pub fn multicall3(&self) -> anyhow::Result<ContractRepr> {
        self.load_l1("dev-contracts/Multicall3.sol/Multicall3.json")
    }

    pub fn verifier(&self) -> anyhow::Result<ContractRepr> {
        todo!()
    }
}
