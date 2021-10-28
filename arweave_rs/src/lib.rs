use async_trait::async_trait;
use infer;
use log::debug;
use num_bigint::BigUint;
use reqwest::{
    self,
    header::{ACCEPT, CONTENT_TYPE},
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    str::FromStr,
    time::{Duration, SystemTime},
};
use tokio::{fs::File, io::AsyncReadExt};
use url::Url;

pub mod crypto;
pub mod error;
pub mod merkle;
pub mod transaction;

use crypto::Methods as CryptoMethods;
use merkle::{generate_data_root, generate_leaves, resolve_proofs};
use transaction::{Base64, FromStrs, Tag, Transaction};

pub type Error = Box<dyn std::error::Error>;

pub struct Arweave {
    pub name: String,
    pub units: String,
    pub base_url: Url,
    pub crypto: crypto::Provider,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct RawStatus {
    block_height: u64,
    block_indep_hash: Base64,
    number_of_confirmations: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StatusCode {
    Submitted,
    NotFound,
    Pending,
    Confirmed,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Status {
    path: PathBuf,
    id: Base64,
    created_at: Duration,
    last_modified: Duration,
    status: StatusCode,
    #[serde(flatten)]
    raw_status: RawStatus,
}

#[derive(Serialize, Deserialize, Debug)]
struct OraclePrice {
    pub arweave: OraclePricePair,
}

#[derive(Serialize, Deserialize, Debug)]
struct OraclePricePair {
    pub usd: f32,
}

#[async_trait]
pub trait Methods<T> {
    async fn from_keypair_path(keypair_path: PathBuf, base_url: Option<&str>) -> Result<T, Error>;
    async fn get_wallet_balance(&self, wallet_address: Option<String>) -> Result<BigUint, Error>;
    async fn get_price(&self, bytes: &usize) -> Result<(BigUint, BigUint), Error>;
    async fn get_transaction(&self, id: &Base64) -> Result<Transaction, Error>;
    async fn create_transaction_from_file_path(
        &self,
        file_path: PathBuf,
        other_tags: Option<Vec<Tag>>,
        last_tx: Option<Base64>,
        reward: Option<u64>,
    ) -> Result<Transaction, Error>;
    fn sign_transaction(&self, transaction: Transaction) -> Result<Transaction, Error>;
    async fn post_transaction(
        &self,
        transaction: &Transaction,
        manifest_dir: Option<PathBuf>,
    ) -> Result<(), Error>;
    async fn check_status(&self, id: &Base64) -> Result<Status, Error>;
}

#[async_trait]
impl Methods<Arweave> for Arweave {
    async fn from_keypair_path(
        keypair_path: PathBuf,
        base_url: Option<&str>,
    ) -> Result<Arweave, Error> {
        Ok(Arweave {
            name: String::from("arweave"),
            units: String::from("winstons"),
            base_url: Url::parse(base_url.unwrap_or("https://arweave.net/"))?,
            crypto: crypto::Provider::from_keypair_path(keypair_path).await?,
        })
    }

    /// Returns the balance of the wallet.
    async fn get_wallet_balance(&self, wallet_address: Option<String>) -> Result<BigUint, Error> {
        let wallet_address = if let Some(wallet_address) = wallet_address {
            wallet_address
        } else {
            self.crypto.wallet_address()?.to_string()
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
    async fn get_transaction(&self, id: &Base64) -> Result<Transaction, Error> {
        let url = self.base_url.join("tx/")?.join(&id.to_string())?;
        let resp = reqwest::get(url).await?.json::<Transaction>().await?;
        Ok(resp)
    }

    async fn create_transaction_from_file_path(
        &self,
        file_path: PathBuf,
        other_tags: Option<Vec<Tag>>,
        last_tx: Option<Base64>,
        reward: Option<u64>,
    ) -> Result<Transaction, Error> {
        let mut file = File::open(file_path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;

        let chunks = generate_leaves(data.clone(), &self.crypto)?;
        let root = generate_data_root(chunks.clone(), &self.crypto)?;
        let data_root = Base64(root.id.clone().into_iter().collect());
        let proofs = resolve_proofs(root, None)?;
        let owner = self.crypto.keypair_modulus()?;

        // Get content type from [magic numbers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types)
        // and include additional tags if any.
        let content_type = if let Some(kind) = infer::get(&data) {
            kind.mime_type()
        } else {
            "application/json"
        };
        let mut tags = vec![Tag::from_utf8_strs("Content-Type", content_type)?];

        // Add other tags if provided.
        if let Some(other_tags) = other_tags {
            tags.extend(other_tags);
        }

        // Fetch and set last_tx if not provided (primarily for testing).
        let last_tx = if let Some(last_tx) = last_tx {
            last_tx
        } else {
            let last_tx_str = reqwest::get(self.base_url.join("tx_anchor")?)
                .await?
                .text()
                .await?;
            Base64::from_str(&last_tx_str)?
        };

        // Fetch and set reward if not provided (primarily for testing).
        let reward = reward.unwrap_or({
            let (winstons_per_bytes, _) = self.get_price(&data.len()).await?;
            winstons_per_bytes.to_u64_digits()[0]
        });

        Ok(Transaction {
            format: 2,
            data_size: data.len() as u64,
            data: Base64(data),
            data_root,
            tags,
            reward,
            owner,
            last_tx,
            chunks,
            proofs,
            ..Default::default()
        })
    }

    /// Gets deep hash, signs and sets signature and id.
    fn sign_transaction(&self, mut transaction: Transaction) -> Result<Transaction, Error> {
        let deep_hash = self.crypto.deep_hash(&transaction)?;
        let signature = self.crypto.sign(&deep_hash)?;
        let id = self.crypto.hash_SHA256(&signature)?;
        transaction.signature = Base64(signature);
        transaction.id = Base64(id.to_vec());
        Ok(transaction)
    }

    async fn post_transaction(
        &self,
        transaction: &Transaction,
        manifest_dir: Option<PathBuf>,
    ) -> Result<(), Error> {
        let url = self.base_url.join("tx/")?;
        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .json(&transaction)
            .header(&ACCEPT, "application/json")
            .header(&CONTENT_TYPE, "application/json")
            .send()
            .await?;
        debug!("post_transaction {:?}", &resp);
        assert_eq!(resp.status().as_u16(), 200);
        println!(
            "Posted transaction: {}{}",
            self.base_url.to_string(),
            transaction.id
        );
        Ok(())
    }

    async fn check_status(&self, id: &Base64) -> Result<Status, Error> {
        let url = self.base_url.join(&format!("tx/{}/status", id))?;
        let resp = reqwest::get(url).await?;
        println!("{:?}", resp);
        let resp = resp.json::<Status>().await?;
        Ok(resp)
    }
}
