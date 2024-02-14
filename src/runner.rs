use xshell::Shell;

#[derive(Debug)]
pub(crate) struct InitContext {
    pub base_folder: String,
    pub web3_network: String,
    pub need_postgres: bool,
    pub postgres_url: String,
    pub need_geth: bool,
}

#[derive(Debug)]
struct Runner {
    shell: Shell,
    context: InitContext,
}

impl Runner {
    pub fn new(shell: Shell, context: InitContext) -> Self {
        Self { shell, context }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
