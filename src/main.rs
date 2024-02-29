#![allow(dead_code)]

use std::{collections::HashMap, time::SystemTime};

use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use derive_more::Display;
use init::Init;
use prerequisites::check_prerequisites;
use serde::{Deserialize, Serialize};
use web3::ethabi::Address;
use xshell::{cmd, Shell};

mod consts;
mod contracts;
mod init;
mod prerequisites;
mod utils;
mod web3_client;

use consts::{DOCKER_IMAGE, DOCKER_TAG, GIT_REPO, GIT_REVISION};

use crate::{consts::localhost_rich_wallet, contracts::Contracts, web3_client::Web3Client};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    version: String,
}

/// Manager for ZK Stack hyperchains
#[derive(Parser, Debug)]
#[command(name = consts::APP_NAME)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Either initializes a new hyperchain or continues an existing
    /// initialization process.
    Init {
        /// Name of the hyperchain.
        name: String,
        #[arg(long)]
        /// L1 network
        l1: L1Network,
        /// L2 chain ID
        #[arg(long)]
        chain_id: u64,
        /// (Sepolia only) URL of Web3 API
        #[arg(long, required_if_eq("l1", "sepolia"))]
        web3_rpc: Option<url::Url>,
    },
    /// Prints the location for a certain hyperchain data.
    Where {
        /// Name of the hyperchain.
        name: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrerequisiteContracts {
    pub multicall3: Address,
    pub create2_factory: Address,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    human_panic::setup_panic!();

    let opts = Cli::parse();
    match opts.command {
        Commands::Init {
            name,
            l1,
            chain_id,
            web3_rpc,
        } => init(name, l1, chain_id, web3_rpc).await,
        Commands::Where { name } => {
            let dir = utils::hyperchain_dir(&name)?.to_string_lossy().to_string();
            println!("{}", dir);
            Ok(())
        }
    }
}

async fn init_base_dir(shell: &Shell) -> anyhow::Result<()> {
    let base_dir = utils::base_dir()?;
    if shell.path_exists(base_dir.join(".ok")) {
        // TODO: Check contents
        return Ok(());
    }
    shell.create_dir(&base_dir)?;

    // Clone main repo and checkout to desired revision.
    println!("Cloning core repository...");
    let repo_dir = base_dir.join(".repo");
    if shell.path_exists(&repo_dir) {
        shell.remove_path(&repo_dir)?;
    }
    shell.change_dir(&base_dir);
    cmd!(shell, "git clone {GIT_REPO} .repo").output()?; // TODO: retry?
    shell.change_dir(&repo_dir);
    cmd!(shell, "git checkout {GIT_REVISION}").output()?; // TODO: retry?
    println!("Repository cloned");

    // Copy contracts from server docker image to the base folder.
    println!("Copying contracts from the docker image...");
    cmd!(
        shell,
        "docker pull --platform linux/amd64 {DOCKER_IMAGE}:{DOCKER_TAG}"
    )
    .run()?; // TODO: retry?
    let dummy_container_name = format!(
        "zksync-dummy-{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
    );
    cmd!(
        shell,
        "docker create --platform linux/amd64 --name {dummy_container_name} {DOCKER_IMAGE}:{DOCKER_TAG}"
    )
    .run()?;
    let contracts_dir = base_dir.join(".contracts");
    if shell.path_exists(&contracts_dir) {
        shell.remove_path(&contracts_dir)?;
    }
    cmd!(
        shell,
        "docker cp {dummy_container_name}:/contracts {contracts_dir}"
    )
    .run()?;
    cmd!(shell, "docker rm -f {dummy_container_name}").run()?;

    // Docker volumes.
    println!("Creating folders for docker volumes...");
    shell.create_dir(base_dir.join("volumes"))?;
    shell.create_dir(base_dir.join("volumes/geth"))?;
    shell.create_dir(base_dir.join("volumes/postgres"))?;

    // Docker-compose
    println!("Copying docker-compose template...");
    const DOCKER_COMPOSE_FILE: &'static str =
        include_str!("../assets/docker-compose-template.yaml");
    shell.write_file(base_dir.join("docker-compose.yaml"), DOCKER_COMPOSE_FILE)?;

    // Deploy multicall3 and create2factory on localhost.
    start_containers(&shell)?;
    println!("Deploying prerequisite contracts to localhost L1");
    let deployer = Web3Client::new(init::LOCALHOST_WEB3)?;
    deployer.wait_until_up().await?;
    let contracts = Contracts::new(&shell, base_dir.clone());
    let multicall3 = contracts.multicall3().context("load multicall3")?;
    let create2_factory = contracts
        .create2_factory()
        .context("load create2_factory")?;
    let multicall3 = deployer
        .deploy(
            localhost_rich_wallet(),
            &multicall3.raw_abi,
            multicall3.bytecode,
            (),
        )
        .await
        .context("deploy multicall3")?;
    println!("Deployed Multicall3 contract to localhost");
    let create2_factory = deployer
        .deploy(
            localhost_rich_wallet(),
            &create2_factory.raw_abi,
            create2_factory.bytecode,
            (),
        )
        .await
        .context("deploy create2 factory")?;
    println!("Deployed Create2 contract to localhost");
    let mut prerequisite_contracts = HashMap::new();
    prerequisite_contracts.insert(
        L1Network::Localhost,
        PrerequisiteContracts {
            create2_factory: create2_factory.address(),
            multicall3: multicall3.address(),
        },
    );
    prerequisite_contracts.insert(
        L1Network::Sepolia,
        PrerequisiteContracts {
            create2_factory: "ce0042b868300000d44a59004da54a005ffdcf9f".parse().unwrap(),
            multicall3: "cA11bde05977b3631167028862bE2a173976CA11".parse().unwrap(),
        },
    );
    let encoded_contracts = serde_json::to_string_pretty(&prerequisite_contracts).unwrap();
    shell.write_file(
        base_dir.join(".prerequisite_contracts.json"),
        &encoded_contracts,
    )?;

    // Mark the workspace as initialized.
    let meta = WorkspaceMetadata {
        version: "0.0.1".to_string(),
    };
    let encoded_meta = serde_json::to_string_pretty(&meta).unwrap();
    shell.write_file(base_dir.join(".ok"), &encoded_meta)?;
    Ok(())
}

fn start_containers(shell: &Shell) -> anyhow::Result<()> {
    let base_dir = utils::base_dir()?;
    shell.change_dir(base_dir);
    cmd!(shell, "docker-compose up -d zkstack_geth zkstack_postgres").run()?;
    Ok(())
}

async fn init(
    name: String,
    l1_network: L1Network,
    chain_id: u64,
    web3_rpc: Option<url::Url>,
) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    check_prerequisites(&shell);
    init_base_dir(&shell).await?;
    start_containers(&shell)?;

    let init = Init::new(name, l1_network, chain_id, web3_rpc)?;
    init.init().await?;

    Ok(())
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize, Display, ValueEnum)]
enum L1Network {
    Localhost,
    Sepolia,
}
