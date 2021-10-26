use async_trait::async_trait;
use infer;
use log::debug;
use num_bigint::BigUint;
use reqwest::{
    self,
    header::{ACCEPT, CONTENT_TYPE},
};
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncReadExt};
use url::Url;

pub mod crypto;
pub mod error;
pub mod merkle;
pub mod transaction;

use crypto::Methods as CryptoMethods;
use merkle::{generate_data_root, generate_leaves, resolve_proofs};
use transaction::{Base64, Transaction};

pub type Error = Box<dyn std::error::Error>;

pub struct Arweave {
    pub name: String,
    pub units: String,
    pub base_url: Url,
    pub crypto: crypto::Provider,
}

#[derive(Serialize, Deserialize, Debug)]
struct OraclePrice {
    pub arweave: OraclePricePair,
}

#[derive(Serialize, Deserialize, Debug)]
struct OraclePricePair {
    pub usd: f32,
}

//

#[async_trait]
pub trait Methods<T> {
    async fn from_keypair_path(keypair_path: &str) -> Result<T, Error>;
    async fn get_wallet_balance(&self, wallet_address: Option<String>) -> Result<BigUint, Error>;
    async fn get_price(&self, bytes: &usize) -> Result<(BigUint, BigUint), Error>;
    async fn get_transaction(&self, id: &str) -> Result<Transaction, Error>;
    async fn create_transaction_from_file_path(
        &self,
        file_path: &str,
    ) -> Result<Transaction, Error>;
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
    async fn get_transaction(&self, id: &str) -> Result<Transaction, Error> {
        let url = self.base_url.join("tx/")?.join(id)?;
        let resp = reqwest::get(url).await?.json::<Transaction>().await?;
        println!("{owner:?}", owner = resp.owner.0);
        Ok(resp)
    }

    async fn create_transaction_from_file_path(
        &self,
        file_path: &str,
    ) -> Result<Transaction, Error> {
        let mut file = File::open(file_path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;

        let chunks = generate_leaves(data.clone(), &self.crypto)?;
        let root = generate_data_root(chunks.clone(), &self.crypto)?;
        let data_root = Base64(root.id.clone().into_iter().collect());
        let proofs = resolve_proofs(root, None)?;
        let owner = self.crypto.keypair_modulus()?;

        Ok(Transaction {
            format: 2,
            data_size: data.len() as u64,
            data: Base64(data),
            data_root,
            chunks,
            proofs,
            owner,
            ..Default::default()
        })
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

// // Calc data_size and encode.
// let data_size = &buffer.len();
// let data = buffer.to_base64_string()?;

// // Get cost of upload as reward and encode
// // along with data_size.
// let reward = self
//     .price(&data_size)
//     .await
//     .and_then(|p| Ok(p.0.to_string()))?;

// let data_size = data_size.to_string();

// // Determine mime type - simplification that anything not identified is
// // application/json - and create tags. Encoded tags needed for
// // calculation of data_root.
// let content_type = if let Some(kind) = infer::get(&buffer) {
//     kind.mime_type()
// } else {
//     "application/json"
// };
// let tag_name = "Content-Type".to_base64_string()?;
// let tag_value = content_type.to_base64_string()?;
// let tags = vec![Tag {
//     name: tag_name,
//     value: tag_value,
// }];
// let serialized_tags = serde_json::to_string(&tags).unwrap();

// // Get tx_acnchor - already encoded.
// let last_tx = reqwest::get(self.base_url.join("tx_anchor").unwrap())
//     .await?
//     .text()
//     .await?;

// // let last_tx = "".to_string();

// // Get owner, same as wallet address.
// // let owner = self
// //     .keypair
// //     .public_key()
// //     .modulus()
// //     .big_endian_without_leading_zero();

// let format = "2".to_string();

// // Include empty strings for quantity and target.
// let quantity = "".to_string();
// let target = "".to_string();

// // Calculate merkle root as data_root.
// let base64_fields = [
//     &format,
//     // &owner,
//     &target,
//     &data_size,
//     &quantity,
//     &reward,
//     &last_tx,
//     &serialized_tags,
// ];
// let hashed_base64_fields =
//     try_join_all(base64_fields.map(|s| hash_sha384(s.as_bytes()))).await?;

// let data_root = &hashed_base64_fields
//     .into_iter()
//     .flatten()
//     .collect::<Vec<u8>>()[..];

// // Sign and encode data_root as id.
// let signature = self.sign(&data_root).await?;

// let id = hash_sha256(&signature.as_ref()).await?.to_base64_string()?;

// // Create transaction.
// let transaction = Transaction {
//     format: 2,
//     id,
//     last_tx,
//     // owner,
//     tags: Some(tags),
//     target: Some(target),
//     quantity: Some(quantity),
//     data_root: data_root.to_base64_string()?,
//     data_size,
//     data,
//     reward,
//     signature: signature.to_base64_string()?,
// };

// debug!("trnsaction {:?}", &transaction);
