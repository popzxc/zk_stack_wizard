use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use sqlx::{
    migrate::{Migrate, MigrateError, Migrator},
    Connection, PgConnection,
};
use web3::types::{H256, U256};
use xshell::Shell;

use crate::{deploy::Deployer, L1Network};

const LOCALHOST_WEB3: &'static str = "http://127.0.0.1:18546";
const BASE_DB_URL: &'static str = "postgres://postgres:notsecurepassword@127.0.0.1:15432";
const STATE_FILE_NAME: &'static str = ".init_state.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct InitState {
    db_name: Option<String>,
    #[serde(default)]
    migrations_applied: bool,
    admin_wallet: Option<H256>,
    operator_wallet: Option<H256>,
    #[serde(default)]
    wallets_funded: bool,
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
        self.migrate_db().await?;
        println!("Migrations applied");

        self.generate_wallets().await?;

        // self.fund_wallets().await?;

        // self.deploy_prerequisites().await?;

        // self.deploy_verifier().await?;

        // self.run_genesis().await?;

        // self.deploy_l1().await?;

        // self.deploy_l2().await?;

        // self.generate_configs().await?;

        // self.modify_docker_compose().await?;

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
            if PgConnection::connect(BASE_DB_URL).await.is_ok() {
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
        let mut connection = PgConnection::connect(BASE_DB_URL).await?;

        let db_name = format!("{}_{}", self.name, self.l1_network).to_lowercase();
        // Who cares about escaping, YOLO.
        let query = format!("CREATE DATABASE {db_name}");
        // Create DB.
        sqlx::query(&query).execute(&mut connection).await?;

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

    async fn migrate_db(&self) -> anyhow::Result<()> {
        // Most of this file is copy-pasted from SQLx CLI:
        // https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/src/migrate.rs
        // Warrants a refactoring if this tool makes it to production.

        let mut state = self.load_state()?;
        if state.migrations_applied {
            return Ok(());
        }

        let migrations_folder = self.base_dir.join(".repo/core/lib/dal/migrations");
        if !self.shell.path_exists(&migrations_folder) {
            anyhow::bail!("Migrations folder doesn't exist");
        }
        let migrator = Migrator::new(migrations_folder).await?;

        let full_db_path = self.full_db_path(&state)?;
        let mut conn = PgConnection::connect(&full_db_path).await?;
        conn.ensure_migrations_table().await?;

        let version = conn.dirty_version().await?;
        if let Some(version) = version {
            anyhow::bail!(MigrateError::Dirty(version));
        }

        let applied_migrations = conn.list_applied_migrations().await?;
        // validate_applied_migrations(&applied_migrations, &migrator, ignore_missing)?;

        let latest_version = applied_migrations
            .iter()
            .max_by(|x, y| x.version.cmp(&y.version))
            .and_then(|migration| Some(migration.version))
            .unwrap_or(0);

        let applied_migrations: HashMap<_, _> = applied_migrations
            .into_iter()
            .map(|m| (m.version, m))
            .collect();

        for migration in migrator.iter() {
            if migration.migration_type.is_down_migration() {
                // Skipping down migrations
                continue;
            }

            match applied_migrations.get(&migration.version) {
                Some(applied_migration) => {
                    if migration.checksum != applied_migration.checksum {
                        anyhow::bail!(MigrateError::VersionMismatch(migration.version));
                    }
                }
                None => {
                    let skip = false;

                    let elapsed = conn.apply(migration).await?;
                    let text = if skip { "Skipped" } else { "Applied" };

                    // TODO: SQLx had nice styiling here.
                    println!(
                        "{} {}/{} {} {}",
                        text,
                        migration.version,
                        migration.migration_type.label(),
                        migration.description,
                        format!("({elapsed:?})")
                    );
                }
            }
        }

        // Close the connection before exiting:
        // * For MySQL and Postgres this should ensure timely cleanup on the server side,
        //   including decrementing the open connection count.
        // * For SQLite this should checkpoint and delete the WAL file to ensure the migrations
        //   were actually applied to the database file and aren't just sitting in the WAL file.
        let _ = conn.close().await;

        state.migrations_applied = true;
        self.save_state(state)?;
        Ok(())
    }

    async fn generate_wallets(&self) -> anyhow::Result<()> {
        let mut state = self.load_state()?;
        if state.operator_wallet.is_some() && state.admin_wallet.is_some() {
            return Ok(());
        }
        state.operator_wallet = Some(crate::deploy::gen_pk());
        state.admin_wallet = Some(crate::deploy::gen_pk());
        self.save_state(state)?;
        Ok(())
    }

    async fn fund_wallets(&self) -> anyhow::Result<()> {
        let mut state = self.load_state()?;
        if state.wallets_funded {
            return Ok(());
        }

        let admin_wallet =
            crate::deploy::address(state.admin_wallet.expect("Must've been initialized"));
        let deployer = Deployer::new(&self.web3_rpc)?;
        let balance = deployer.balance_of(admin_wallet).await?;

        let one_eth = U256::from(10).pow(18.into());

        if balance < one_eth {
            match self.l1_network {
                L1Network::Localhost => {
                    todo!("Fund from rich wallet");
                }
                L1Network::Sepolia => {
                    todo!("Ask users to provide funds");
                }
            }
        }

        todo!("Give operator smth");

        state.wallets_funded = true;
        Ok(())
    }
}
