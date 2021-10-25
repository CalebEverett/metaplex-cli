use async_trait::async_trait;

use futures::future::try_join_all;

use infer;
use jsonwebkey::JsonWebKey;
use log::debug;
use num_bigint::BigUint;
use reqwest::{
    self,
    header::{ACCEPT, CONTENT_TYPE},
};
use ring::{
    digest::{Context, SHA256, SHA384},
    rand,
    signature::{self, KeyPair, RsaKeyPair},
};
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncReadExt};
use url::Url;

pub mod crypto;
pub mod error;
pub mod merkle;
pub mod transaction;
use crypto::Methods as CryptoMethods;
use transaction::Transaction;

type Error = Box<dyn std::error::Error>;

pub struct Arweave {
    pub name: String,
    pub units: String,
    base_url: Url,
    crypto: crypto::Provider,
}

#[derive(Serialize, Deserialize, Debug)]
struct OraclePrice {
    arweave: OraclePricePair,
}

#[derive(Serialize, Deserialize, Debug)]
struct OraclePricePair {
    usd: f32,
}

pub trait EncDec {
    fn decode_base64_bytes(&self) -> Result<Vec<u8>, Error>;
}
pub trait Base64Encode {
    fn to_base64_string(&self) -> Result<String, Error>;
}

impl EncDec for String {
    fn decode_base64_bytes(&self) -> Result<Vec<u8>, Error> {
        base64::decode_config(self, base64::URL_SAFE_NO_PAD).map_err(|e| e.into())
    }
}

impl Base64Encode for String {
    fn to_base64_string(&self) -> Result<String, Error> {
        let base64_string: String = base64::encode_config(self.as_bytes(), base64::URL_SAFE_NO_PAD);
        Ok(base64_string)
    }
}

impl Base64Encode for &str {
    fn to_base64_string(&self) -> Result<String, Error> {
        let base64_string: String = base64::encode_config(self, base64::URL_SAFE_NO_PAD);
        Ok(base64_string)
    }
}

impl Base64Encode for &[u8] {
    fn to_base64_string(&self) -> Result<String, Error> {
        let base64_string = base64::encode_config(self, base64::URL_SAFE_NO_PAD);
        Ok(base64_string)
    }
}

impl Base64Encode for Vec<u8> {
    fn to_base64_string(&self) -> Result<String, Error> {
        let base64_string: String = base64::encode_config(self, base64::URL_SAFE_NO_PAD);
        Ok(base64_string)
    }
}

#[async_trait]
pub trait Methods<T> {
    async fn from_keypair_path(keypair_path: &str) -> Result<T, Error>;
    fn get_wallet_address(&self) -> Result<String, Error>;
    async fn get_wallet_balance(&self, wallet_address: Option<String>) -> Result<BigUint, Error>;
    async fn get_price(&self, bytes: &usize) -> Result<(BigUint, BigUint), Error>;
    async fn get_transaction(&self, id: &str) -> Result<Transaction, Error>;
    async fn create_transaction_from_file_path(&self, file_path: &str) -> Result<(), Error>;
    async fn post_transaction(&self, transaction: &Transaction) -> Result<(), Error>;
}

#[async_trait]
impl Methods<Arweave> for Arweave {
    async fn from_keypair_path(keypair_path: &str) -> Result<Arweave, Error> {
        Ok(Arweave {
            name: String::from("arweave"),
            units: String::from("winstons"),
            base_url: Url::parse("https://arweave.net/")?,
            crypto: crypto::Provider::from_keypair_path(keypair_path).await?,
        })
    }
    /// Calculates the wallet address of the provided keypair according to [addressing](https://docs.arweave.org/developers/server/http-api#addressing)
    /// in documentation.
    ///```
    /// # use ring::{signature, rand};
    /// # use arweave_rs::{Arweave, Methods};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let arweave = Arweave::from_keypair_path("tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json").await?;
    /// let calc = arweave.get_wallet_address()?;
    /// let actual = String::from("7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg");
    /// assert_eq!(&calc, &actual);
    /// # Ok(())
    /// # }
    /// ```
    fn get_wallet_address(&self) -> Result<String, Error> {
        let mut context = Context::new(&SHA256);
        let modulus = self
            .crypto
            .keypair
            .public_key()
            .modulus()
            .big_endian_without_leading_zero();
        context.update(modulus);

        // let wallet_address = base64::encode_config(context.finish(), base64::URL_SAFE_NO_PAD);
        let wallet_address = context.finish().as_ref().to_base64_string()?;
        Ok(wallet_address)
    }

    /// Returns the balance of the wallet.
    async fn get_wallet_balance(&self, wallet_address: Option<String>) -> Result<BigUint, Error> {
        let wallet_address = if let Some(wallet_address) = wallet_address {
            wallet_address
        } else {
            self.get_wallet_address()?
        };
        let url = self
            .base_url
            .join(&format!("wallet/{}/balance", &wallet_address))?;
        let winstons = reqwest::get(url).await?.json::<u64>().await?;
        Ok(BigUint::from(winstons))
    }

    /// Returns price of uploading data to the network in winstons and usd per AR
    /// as a BigUint with two decimals.
    async fn get_price(&self, bytes: &usize) -> Result<(BigUint, BigUint), Error> {
        let url = self.base_url.join("price/")?.join(&bytes.to_string())?;
        let winstons_per_bytes = reqwest::get(url).await?.json::<u64>().await?;
        let winstons_per_bytes = BigUint::from(winstons_per_bytes);
        let oracle_url =
            "https://api.coingecko.com/api/v3/simple/price?ids=arweave&vs_currencies=usd";
        let usd_per_ar = reqwest::get(oracle_url)
            .await?
            .json::<OraclePrice>()
            .await?
            .arweave
            .usd;

        let usd_per_ar: BigUint = BigUint::from((usd_per_ar * 100.0).floor() as u32);

        Ok((winstons_per_bytes, usd_per_ar))
    }
    async fn get_transaction(&self, id: &str) -> Result<Transaction, Error> {
        let url = self.base_url.join("tx/")?.join(id)?;
        let resp = reqwest::get(url).await?.json::<Transaction>().await?;
        println!("{owner:?}", owner = resp.owner.0);
        Ok(resp)
    }

    async fn create_transaction_from_file_path(&self, file_path: &str) -> Result<(), Error> {
        // Read file to buffer
        let mut file = File::open(file_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;
        Ok(())
    }

    async fn post_transaction(&self, transaction: &Transaction) -> Result<(), Error> {
        let url = self.base_url.join("tx/")?;
        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .json(&transaction)
            .header(&ACCEPT, "application/json")
            .header(&CONTENT_TYPE, "application/json")
            .send()
            .await?;
        debug!("trnsaction {:?}", &resp.url());
        assert_eq!(resp.status().as_u16(), 200);
        println!("Posted transaction: https://arweave.net/{}", transaction.id);
        Ok(())
    }
}
