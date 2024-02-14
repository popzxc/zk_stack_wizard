//! Version specification for the deployed hyperchain
//! Denotes stuff like server docker image, repo commit hashes, etc.

#[derive(Debug)]
#[non_exhaustive]
pub(super) struct HyperchainVersion {}

impl HyperchainVersion {
    pub fn new() -> Self {
        Self {}
    }

    pub fn server_docker_image(&self) -> &'static str {
        "matterlabs/server-v2"
    }

    pub fn server_docker_tag(&self) -> &'static str {
        "bd63b3a-1707838915921"
    }

    pub fn server_git_repo(&self) -> &'static str {
        "https://github.com/matter-labs/zksync-era.git"
    }

    pub fn server_git_revision(&self) -> &'static str {
        "bd63b3a"
    }
}
