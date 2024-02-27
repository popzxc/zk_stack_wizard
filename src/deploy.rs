use web3::{
    ethabi::Address,
    transports::Http,
    types::{H256, U256},
    Web3,
};

pub fn gen_pk() -> H256 {
    use web3::signing::SecretKey;

    loop {
        let pk = H256::random();
        if SecretKey::from_slice(pk.as_bytes()).is_ok() {
            return pk;
        }
    }
}

pub fn address(pk: H256) -> Address {
    use web3::signing::{Key, SecretKey, SecretKeyRef};

    let sk = SecretKey::from_slice(pk.as_bytes()).expect("Bad pk");
    SecretKeyRef::new(&sk).address()
}

#[derive(Debug)]
pub struct Deployer {
    web3_client: Web3<Http>,
}

impl Deployer {
    pub fn new(web3_url: &str) -> anyhow::Result<Self> {
        let transport = Http::new(web3_url)?;
        Ok(Self {
            web3_client: Web3::new(transport),
        })
    }

    pub async fn balance_of(&self, address: Address) -> anyhow::Result<U256> {
        let balance = self.web3_client.eth().balance(address, None).await?;
        Ok(balance)
    }
}
