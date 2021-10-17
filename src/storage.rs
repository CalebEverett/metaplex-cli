use async_trait::async_trait;
use base64;
use log::debug;
use reqwest;
use url::Url;

use crate::Error;

pub struct Provider {
    pub name: String,
    pub units: String,
    base: Url,
}
#[async_trait]
pub trait Methods {
    async fn get_price(&self, bytes: u32) -> Result<u64, Error>;
}

struct Transaction {
    format: u8,
    id: String,
    last_tx: String,
    owner: String,
}

pub fn get_provider(provider: &str) -> Result<Provider, Error> {
    match provider {
        "arweave" => {
            let provider = Provider {
                name: provider.to_string(),
                units: "winstons".to_string(),
                base: Url::parse("https://arweave.net/")?,
            };
            assert!(!provider.base.cannot_be_a_base());

            #[async_trait]
            impl Methods for Provider {
                async fn get_price(&self, bytes: u32) -> Result<u64, Error> {
                    let url = self.base.join("price/")?.join(&bytes.to_string())?;
                    debug!("url: {}", url);
                    let resp = reqwest::get(url).await?.json::<u64>().await?;
                    Ok(resp)
                }
            }
            Ok(provider)
        }
        _ => unreachable!(),
    }
}
