use std::time::Duration;

use web3::{
    contract::{tokens::Tokenize, Contract},
    ethabi::{self, Address},
    signing::{Key, SecretKey, SecretKeyRef},
    transports::Http,
    types::{H256, U256},
    Web3,
};

pub fn gen_pk() -> H256 {
    loop {
        let pk = H256::random();
        if SecretKey::from_slice(pk.as_bytes()).is_ok() {
            return pk;
        }
    }
}

pub fn address(pk: H256) -> Address {
    let sk = SecretKey::from_slice(pk.as_bytes()).expect("Bad pk");
    SecretKeyRef::new(&sk).address()
}

#[derive(Debug)]
pub struct Web3Client {
    url: String,
    web3_client: Web3<Http>,
}

impl Web3Client {
    pub fn new(web3_url: &str) -> anyhow::Result<Self> {
        let transport = Http::new(web3_url)?;
        Ok(Self {
            url: web3_url.to_string(),
            web3_client: Web3::new(transport),
        })
    }

    /// Tries to wait until corresponding Web3 is up and running.
    pub async fn wait_until_up(&self) -> anyhow::Result<()> {
        // 100 retries with 200ms frequency give us 20 seconds to wait.
        for _ in 0..100 {
            if self.chain_id().await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        anyhow::bail!("Web3 RPC with URL {} does not respond", self.url);
    }

    pub async fn chain_id(&self) -> anyhow::Result<U256> {
        let id = self.web3_client.eth().chain_id().await?;
        Ok(id)
    }

    pub async fn balance_of(&self, address: Address) -> anyhow::Result<U256> {
        let balance = self.web3_client.eth().balance(address, None).await?;
        Ok(balance)
    }

    pub async fn deploy<P: Tokenize>(
        &self,
        pk: H256,
        json: &[u8],
        bytecode: String,
        constructor_args: P,
    ) -> anyhow::Result<Contract<Http>> {
        let chain_id = None; // TODO should not be none.
        let pk = SecretKey::from_slice(pk.as_bytes()).unwrap();
        let contract = Contract::deploy(self.web3_client.eth(), json)?
            .sign_with_key_and_execute(bytecode, constructor_args, &pk, chain_id)
            .await?;
        Ok(contract)
    }
}
