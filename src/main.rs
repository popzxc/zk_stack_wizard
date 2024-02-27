#![allow(dead_code)]

use std::time::SystemTime;

use clap::{Parser, Subcommand, ValueEnum};
use derive_more::Display;
use init::Init;
use prerequisites::check_prerequisites;
use serde::{Deserialize, Serialize};
use xshell::{cmd, Shell};

mod consts;
mod contracts;
mod deploy;
mod init;
mod prerequisites;
mod utils;

use consts::{DOCKER_IMAGE, DOCKER_TAG, GIT_REPO, GIT_REVISION};

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

fn init_base_dir(shell: &Shell) -> anyhow::Result<()> {
    let base_dir = utils::base_dir()?;
    if shell.path_exists(base_dir.join(".ok")) {
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

    // Mark the workspace as initialized.
    shell.write_file(base_dir.join(".ok"), "v0.0.1")?;
    Ok(())
}

fn start_containers(shell: &Shell) -> anyhow::Result<()> {
    let base_dir = utils::base_dir()?;
    shell.change_dir(base_dir);
    cmd!(shell, "docker-compose up -d geth postgres").run()?;
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
    init_base_dir(&shell)?;
    start_containers(&shell)?;

    let init = Init::new(name, l1_network, chain_id, web3_rpc)?;
    init.init().await?;

    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display, ValueEnum)]
enum L1Network {
    Localhost,
    Sepolia,
}
