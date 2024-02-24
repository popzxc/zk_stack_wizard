use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use xshell::Shell;

use crate::L1Network;

const LOCALHOST_WEB3: &'static str = "http://127.0.0.1:18546";
const DB_URL: &'static str = "postgres://postgres:notsecurepassword@127.0.0.1:15432";
const STATE_FILE_NAME: &'static str = ".init_state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InitState {
    todo: String,
}

pub struct Init {
    name: String,
    l1_network: L1Network,
    chain_id: u64,
    web3_rpc: String,

    shell: Shell,
    base_dir: PathBuf,
    hyperchain_dir: PathBuf,
}

impl Init {
    pub fn new(
        name: String,
        l1_network: L1Network,
        chain_id: u64,
        web3_rpc: Option<url::Url>,
    ) -> anyhow::Result<Self> {
        let web3_rpc = match l1_network {
            L1Network::Localhost => LOCALHOST_WEB3.to_string(),
            L1Network::Sepolia => web3_rpc.unwrap().to_string(),
        };
        let shell = Shell::new()?;
        let base_dir = crate::utils::base_dir()?;
        let hyperchain_dir = crate::utils::hyperchain_dir(&name)?;

        Ok(Self {
            name,
            l1_network,
            chain_id,
            web3_rpc,
            shell,
            base_dir,
            hyperchain_dir,
        })
    }
}