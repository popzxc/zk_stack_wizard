#![allow(dead_code)]

use clap::{Parser, Subcommand};
use xshell::{cmd, Shell};

mod init_context;
mod prerequisites;
mod runner;
mod version;

/// Manager for ZK Stack hyperchains
#[derive(Parser, Debug)]
#[command(name = "zk_stack_wizard")]
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
    },
}

fn main_fallible() -> anyhow::Result<()> {
    let args = Cli::parse();

    let shell = Shell::new()?;
    prerequisites::check_prerequisites(&shell);

    // let runner = Runner::new(shell);
    // runner.run()?;
    Ok(())
}

fn main() {
    if let Err(err) = main_fallible() {
        println!("A following error occured:");
        println!("{err:#}");
        println!("Unable to continue, exiting.");
    }
}
