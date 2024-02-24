use web3::{transports::Http, Web3};

#[derive(Debug)]
pub struct Deployer {
    web3_client: Web3<Http>,
}

impl Deployer {
    pub fn new(web3_url: url::Url) -> anyhow::Result<Self> {
        let transport = Http::new(web3_url.as_str())?;
        Ok(Self {
            web3_client: Web3::new(transport),
        })
    }
}
