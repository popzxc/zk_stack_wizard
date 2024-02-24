use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;

use crate::consts;

pub fn base_dir() -> anyhow::Result<PathBuf> {
    let project_dirs =
        ProjectDirs::from("", "", consts::APP_NAME).context("Can't load project dirs")?;

    Ok(project_dirs.data_dir().to_owned())
}

pub fn hyperchain_dir(name: &str) -> anyhow::Result<PathBuf> {
    base_dir().map(|d| d.join(name))
}
