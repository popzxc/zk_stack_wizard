use web3::{
    contract::Contract,
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

    pub async fn deploy(
        &self,
        pk: H256,
        json: &[u8],
        bytecode: String,
        constructor_args: Vec<ethabi::Token>,
    ) -> anyhow::Result<Contract<Http>> {
        let chain_id = None; // TODO should not be none.
        let pk = SecretKey::from_slice(pk.as_bytes()).unwrap();
        let contract = Contract::deploy(self.web3_client.eth(), json)?
            .sign_with_key_and_execute(bytecode, constructor_args, &pk, chain_id)
            .await?;
        Ok(contract)
    }
}
