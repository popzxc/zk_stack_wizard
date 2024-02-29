use web3::types::H256;

pub(super) const DOCKER_IMAGE: &'static str = "matterlabs/server-v2";
pub(super) const DOCKER_TAG: &'static str = "bd63b3a-1707838915921";
pub(super) const GIT_REPO: &'static str = "https://github.com/matter-labs/zksync-era.git";
pub(super) const GIT_REVISION: &'static str = "bd63b3a";
pub(super) const APP_NAME: &'static str = "zk_stack_wizard";

pub fn localhost_rich_wallet() -> H256 {
    // Only available in localhost geth setup.
    const RICH_WALLET: &'static str =
        "7726827caac94a7f9e1b160f7ea819f172f7b6f9d2a97f992c38edeab82d4110";
    RICH_WALLET.parse().unwrap()
}
