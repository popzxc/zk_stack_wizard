use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
use xshell::Shell;

use crate::L1Network;

const LOCALHOST_WEB3: &'static str = "http://127.0.0.1:18546";
const BASE_DB_URL: &'static str = "postgres://postgres:notsecurepassword@127.0.0.1:15432";
const STATE_FILE_NAME: &'static str = ".init_state.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct InitState {
    db_name: Option<String>,
    #[serde(default)]
    migrations_applied: bool,
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

    pub async fn init(self) -> anyhow::Result<()> {
        self.wait_for_db().await?;

        println!("Initializing DB");
        self.init_db().await?;
        println!("DB Initialized");

        println!("Running migrations");
        self.migrade_db().await?;
        println!("Migrations applied");

        Ok(())
    }

    fn load_state(&self) -> anyhow::Result<InitState> {
        if !self
            .shell
            .path_exists(self.hyperchain_dir.join(STATE_FILE_NAME))
        {
            return Ok(InitState::default());
        }

        let contents = self
            .shell
            .read_file(self.hyperchain_dir.join(STATE_FILE_NAME))?;
        Ok(serde_json::from_str(&contents)?)
    }

    fn save_state(&self, state: InitState) -> anyhow::Result<()> {
        let contents = serde_json::to_string_pretty(&state)?;
        self.shell
            .write_file(self.hyperchain_dir.join(STATE_FILE_NAME), contents)?;
        Ok(())
    }

    async fn wait_for_db(&self) -> anyhow::Result<()> {
        // We may get here right after we've started the containers, so we may need
        // to wait for db to go up.
        for _ in 0..30 {
            if tokio_postgres::connect(BASE_DB_URL, tokio_postgres::NoTls)
                .await
                .is_ok()
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        anyhow::bail!(
            "Unable to connect to Postgres, connection cannot be established: {}",
            BASE_DB_URL
        );
    }

    async fn init_db(&self) -> anyhow::Result<()> {
        let mut state = self.load_state()?;
        if state.db_name.is_some() {
            return Ok(());
        }

        // Connect to the database.
        let (client, connection) =
            tokio_postgres::connect(BASE_DB_URL, tokio_postgres::NoTls).await?;

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        let db_name = format!("{}_{}", self.name, self.l1_network).to_lowercase();
        // Who cares about escaping, YOLO.
        let query = format!("CREATE DATABASE {db_name}");
        // Crete DB.
        client.execute(&query, &[]).await?;

        state.db_name = Some(db_name);
        self.save_state(state)?;
        Ok(())
    }

    /// Path to postgres that includes the database name.
    fn full_db_path(&self, state: &InitState) -> anyhow::Result<String> {
        let Some(db_name) = &state.db_name else {
            anyhow::bail!("DB is not initialized but attempted to get full path to it");
        };
        Ok(format!("{}/{}", BASE_DB_URL, db_name))
    }

    async fn migrade_db(&self) -> anyhow::Result<()> {
        let mut state = self.load_state()?;
        if state.migrations_applied {
            return Ok(());
        }

        // Connect to the database.
        let db_url = self.full_db_path(&state)?;
        let (mut client, connection) =
            tokio_postgres::connect(&db_url, tokio_postgres::NoTls).await?;

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        // Load migrations.
        let migrations_folder = self.base_dir.join(".repo/core/lib/dal/migrations");
        if !self.shell.path_exists(&migrations_folder) {
            anyhow::bail!("Migrations folder doesn't exist");
        }
        let mut migration_files: Vec<PathBuf> = self
            .shell
            .read_dir(migrations_folder)?
            .into_iter()
            .filter(|p| p.as_os_str().to_str().unwrap().ends_with("up.sql"))
            .collect();
        migration_files.sort_unstable();

        println!("Applying {} migrations...", migration_files.len());

        let txn = client.transaction().await?;

        for migration_file in migration_files {
            let migration = self.shell.read_file(&migration_file)?;
            txn.batch_execute(&migration).await?;
            println!(
                "Applied migration {:?}",
                migration_file.file_name().unwrap()
            );
        }

        txn.commit().await?;

        state.migrations_applied = true;
        self.save_state(state)?;
        Ok(())
    }
}
