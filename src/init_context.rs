use crate::runner::InitContext;
use anyhow::Context as _;
use dialoguer::Input;
use directories::ProjectDirs;
use xshell::Shell;

const PROGRAM_NAME: &'static str = "zk_stack_wizard";

pub(crate) fn find_hyperchain(shell: &Shell, hyperchain: String) -> anyhow::Result<()> {
    let project_dirs = ProjectDirs::from("", "", PROGRAM_NAME)
        .context("Unable to initialize project directory")?;
    let data_dir = project_dirs.data_dir();
    if !shell.path_exists(data_dir) {
        println!("Looks like data folder for {PROGRAM_NAME} does not exist yet");
        println!("Try initializing at least one hyperchain");
        return Ok(());
    }
    let hyperchain_path = data_dir.join(&hyperchain);
    if !shell.path_exists(&hyperchain_path) {
        println!("Looks like {hyperchain} have not been initialized");
        let hyperchains = shell.read_dir(data_dir)?;
        if hyperchains.is_empty() {
            println!("There are no known hyperchains at this moment");
            println!("Try initializing one");
            return Ok(());
        }
        println!("Available hyperchains:");
        for chain in hyperchains {
            let chain_name = chain.components().last().unwrap().as_os_str();
            println!(" - {chain_name:?} at {chain:?}");
        }
        return Ok(());
    }
    println!("{hyperchain} is located at {hyperchain_path:?}");

    Ok(())
}

pub(crate) fn init_context(shell: &Shell, hyperchain: String) -> anyhow::Result<InitContext> {
    let project_dirs = ProjectDirs::from("", "", PROGRAM_NAME)
        .context("Unable to initialize project directory")?;
    let data_dir = project_dirs.data_dir();
    if !shell.path_exists(data_dir) {
        shell
            .create_dir(data_dir)
            .with_context(|| format!("Unable to create data directory at {data_dir:?}"))?;
        println!("Initialized data directory at {data_dir:?}");
        println!("All the future hyperchains init data would be stored there");
    }
    let hyperchain_path = data_dir.join(&hyperchain);
    if !shell.path_exists(&hyperchain_path) {
        shell
            .create_dir(&hyperchain_path)
            .with_context(|| format!("Unable to create data directory at {hyperchain_path:?}"))?;
        println!("Initialized hyperchain directory for {hyperchain} at {hyperchain_path:?}");
    }

    todo!()
}
