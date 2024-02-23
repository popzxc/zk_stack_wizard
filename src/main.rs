#![allow(dead_code)]

use std::{f32::consts::E, path::PathBuf, time::SystemTime};

use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use derive_more::Display;
use directories::ProjectDirs;
use prerequisites::check_prerequisites;
use serde::{Deserialize, Serialize};
use xshell::{cmd, Shell};

mod consts;
mod prerequisites;

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

fn main() -> anyhow::Result<()> {
    human_panic::setup_panic!();

    let opts = Cli::parse();
    match opts.command {
        Commands::Init {
            name,
            l1,
            chain_id,
            web3_rpc,
        } => init(name, l1, chain_id, web3_rpc),
        Commands::Where { name } => {
            let dir = hyperchain_dir(name)?.to_string_lossy().to_string();
            println!("{}", dir);
            Ok(())
        }
    }
}

fn hyperchain_dir(name: String) -> anyhow::Result<PathBuf> {
    let project_dirs =
        ProjectDirs::from("", "", consts::APP_NAME).context("Can't load project dirs")?;

    Ok(project_dirs.data_dir().join(&name))
}

fn init(
    name: String,
    l1_network: L1Network,
    chain_id: u64,
    web3_rpc: Option<url::Url>,
) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    check_prerequisites(&shell);

    let hyperchain_dir = hyperchain_dir(name)?;
    shell.create_dir(&hyperchain_dir)?;

    println!("Cloning core repository...");
    let repo_dir = hyperchain_dir.join(".repo");
    if shell.path_exists(&repo_dir) {
        shell.remove_path(&repo_dir)?;
    }
    shell.change_dir(&hyperchain_dir);
    cmd!(shell, "git clone {GIT_REPO} .repo").output()?; // TODO: retry?
    shell.change_dir(&repo_dir);
    cmd!(shell, "git checkout {GIT_REVISION}").output()?; // TODO: retry?
    println!("Repository cloned");

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
    let contracts_dir = hyperchain_dir.join(".contracts");
    if shell.path_exists(&contracts_dir) {
        shell.remove_path(&contracts_dir)?;
    }
    cmd!(
        shell,
        "docker cp {dummy_container_name}:/contracts {contracts_dir}"
    )
    .run()?;
    cmd!(shell, "docker rm -f {dummy_container_name}").run()?;

    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display, ValueEnum)]
enum L1Network {
    Localhost,
    Sepolia,
}
