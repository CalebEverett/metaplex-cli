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

type Error = Box<dyn std::error::Error>;

pub struct Provider {
    pub name: String,
    pub units: String,
    base_url: Url,
    pub keypair: RsaKeyPair,
}

#[derive(Serialize, Deserialize, Debug)]
struct OraclePrice {
    arweave: OraclePricePair,
}

#[derive(Serialize, Deserialize, Debug)]
struct OraclePricePair {
    usd: f32,
}

pub async fn get_provider(keypair_path: &str) -> Result<Provider, Error> {
    Ok(Provider {
        name: String::from("arweave"),
        units: String::from("winstons"),
        base_url: Url::parse("https://arweave.net/")?,
        keypair: get_keypair(keypair_path).await?,
    })
}

async fn get_keypair(keypair_path: &str) -> Result<RsaKeyPair, Error> {
    debug!("{:?}", keypair_path);
    let mut file = File::open(keypair_path).await?;
    let mut jwk_str = String::new();
    file.read_to_string(&mut jwk_str).await?;
    let jwk_parsed: JsonWebKey = jwk_str.parse().unwrap();
    let keypair = signature::RsaKeyPair::from_pkcs8(&jwk_parsed.key.as_ref().to_der())?;
    Ok(keypair)
}

async fn hash_sha256(message: &[u8]) -> Result<Vec<u8>, Error> {
    let mut context = Context::new(&SHA256);
    context.update(message);
    Ok(context.finish().as_ref().to_vec())
}

async fn hash_sha384(message: &[u8]) -> Result<Vec<u8>, Error> {
    let mut context = Context::new(&SHA384);
    context.update(message);
    Ok(context.finish().as_ref().to_vec())
}

#[async_trait]
pub trait EncDec {
    fn decode_base64_bytes(&self) -> Result<Vec<u8>, Error>;
}
pub trait Base64Encode {
    fn to_base64_string(&self) -> Result<String, Error>;
}

#[async_trait]
impl EncDec for String {
    fn decode_base64_bytes(&self) -> Result<Vec<u8>, Error> {
        base64::decode_config(self, base64::URL_SAFE_NO_PAD).map_err(|e| e.into())
    }
}

#[async_trait]
impl Base64Encode for String {
    fn to_base64_string(&self) -> Result<String, Error> {
        let base64_string: String = base64::encode_config(self.as_bytes(), base64::URL_SAFE_NO_PAD);
        Ok(base64_string)
    }
}

#[async_trait]
impl Base64Encode for &str {
    fn to_base64_string(&self) -> Result<String, Error> {
        let base64_string: String = base64::encode_config(self, base64::URL_SAFE_NO_PAD);
        Ok(base64_string)
    }
}

#[async_trait]
impl Base64Encode for &[u8] {
    fn to_base64_string(&self) -> Result<String, Error> {
        let base64_string = base64::encode_config(self, base64::URL_SAFE_NO_PAD);
        Ok(base64_string)
    }
}

#[async_trait]
impl Base64Encode for Vec<u8> {
    fn to_base64_string(&self) -> Result<String, Error> {
        let base64_string: String = base64::encode_config(self, base64::URL_SAFE_NO_PAD);
        Ok(base64_string)
    }
}

#[async_trait]
pub trait Methods {
    async fn wallet_address(&self) -> Result<String, Error>;
    async fn wallet_balance(&self, wallet_address: Option<String>) -> Result<BigUint, Error>;
    async fn price(&self, bytes: &usize) -> Result<(BigUint, BigUint), Error>;
    async fn get_transaction(&self, id: &str) -> Result<(), Error>;
    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error>;
    async fn verify_signature(&self, signature: &[u8], message: &[u8]) -> Result<(), Error>;
    async fn transaction_from_filepath(&self, filepath: &str) -> Result<Transaction, Error>;
    async fn post_transaction(&self, transaction: &Transaction) -> Result<(), Error>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    format: u8,
    id: String,
    last_tx: String,
    // owner: String,
    tags: Option<Vec<Tag>>,
    target: Option<String>,
    quantity: Option<String>,
    data_root: String,
    data: String,
    data_size: String,
    reward: String,
    signature: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Tag {
    name: String,
    value: String,
}

#[async_trait]
impl Methods for Provider {
    /// Calculates the wallet address of the provided keypair according to [addressing](https://docs.arweave.org/developers/server/http-api#addressing)
    /// in documentation.
    ///```
    /// # use ring::{signature, rand};
    /// # use metaplex_cli_lib::arweave::*;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = get_provider("tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json").await?;
    /// let calc = provider.wallet_address().await?;
    /// let actual = String::from("7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg");
    /// assert_eq!(&calc, &actual);
    /// # Ok(())
    /// # }
    /// ```
    async fn wallet_address(&self) -> Result<String, Error> {
        let mut context = Context::new(&SHA256);
        let modulus = self
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
    async fn wallet_balance(&self, wallet_address: Option<String>) -> Result<BigUint, Error> {
        let wallet_address = if let Some(wallet_address) = wallet_address {
            wallet_address
        } else {
            self.wallet_address().await?
        };
        let url = self
            .base_url
            .join(&format!("wallet/{}/balance", &wallet_address))?;
        let winstons = reqwest::get(url).await?.json::<u64>().await?;
        Ok(BigUint::from(winstons))
    }

    /// Returns price of uploading data to the network in winstons and usd per AR
    /// as a BigUint with two decimals.
    async fn price(&self, bytes: &usize) -> Result<(BigUint, BigUint), Error> {
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
    async fn get_transaction(&self, id: &str) -> Result<(), Error> {
        let url = self.base_url.join("tx/")?.join(id)?;
        let resp = reqwest::get(url).await?.json::<Transaction>().await?;
        debug!("{:?}", resp);
        Ok(())
    }

    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        let rng = rand::SystemRandom::new();
        let mut signature = vec![0; self.keypair.public_modulus_len()];
        self.keypair
            .sign(&signature::RSA_PSS_SHA256, &rng, message, &mut signature)?;
        Ok(signature)
    }

    /// Verifies that a message was signed by the public key of the Provider.key keypair.
    ///```
    /// # use ring::{signature, rand};
    /// # use metaplex_cli_lib::arweave::*;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = get_provider("tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json").await?;
    /// let message = String::from("hello, world");
    /// let rng = rand::SystemRandom::new();
    /// let signature = provider.sign(&message.as_bytes()).await?;
    ///
    /// assert_eq!((), provider.verify_signature(&signature.as_ref(), &message.as_bytes()).await?);
    /// # Ok(())
    /// # }
    /// ```
    async fn verify_signature(&self, signature: &[u8], message: &[u8]) -> Result<(), Error> {
        let public_key = signature::UnparsedPublicKey::new(
            &signature::RSA_PSS_2048_8192_SHA256,
            self.keypair.public_key().as_ref(),
        );
        public_key.verify(message, signature)?;
        Ok(())
    }

    async fn transaction_from_filepath(&self, filepath: &str) -> Result<Transaction, Error> {
        // Read file to buffer
        let mut file = File::open(filepath).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        // Calc data_size and encode.
        let data_size = &buffer.len();
        let data = buffer.to_base64_string()?;

        // Get cost of upload as reward and encode
        // along with data_size.
        let reward = self
            .price(&data_size)
            .await
            .and_then(|p| Ok(p.0.to_string()))?;

        let data_size = data_size.to_string();

        // Determine mime type - simplification that anything not identified is
        // application/json - and create tags. Encoded tags needed for
        // calculation of data_root.
        let content_type = if let Some(kind) = infer::get(&buffer) {
            kind.mime_type()
        } else {
            "application/json"
        };
        let tag_name = "Content-Type".to_base64_string()?;
        let tag_value = content_type.to_base64_string()?;
        let tags = vec![Tag {
            name: tag_name,
            value: tag_value,
        }];
        let serialized_tags = serde_json::to_string(&tags).unwrap();

        // Get tx_acnchor - already encoded.
        let last_tx = reqwest::get(self.base_url.join("tx_anchor").unwrap())
            .await?
            .text()
            .await?;

        // let last_tx = "".to_string();

        // Get owner, same as wallet address.
        // let owner = self
        //     .keypair
        //     .public_key()
        //     .modulus()
        //     .big_endian_without_leading_zero();

        let format = "2".to_string();

        // Include empty strings for quantity and target.
        let quantity = "".to_string();
        let target = "".to_string();

        // Calculate merkle root as data_root.
        let base64_fields = [
            &format,
            // &owner,
            &target,
            &data_size,
            &quantity,
            &reward,
            &last_tx,
            &serialized_tags,
        ];
        let hashed_base64_fields =
            try_join_all(base64_fields.map(|s| hash_sha384(s.as_bytes()))).await?;

        let data_root = &hashed_base64_fields
            .into_iter()
            .flatten()
            .collect::<Vec<u8>>()[..];

        // Sign and encode data_root as id.
        let signature = self.sign(&data_root).await?;

        let id = hash_sha256(&signature.as_ref()).await?.to_base64_string()?;

        // Create transaction.
        let transaction = Transaction {
            format: 2,
            id,
            last_tx,
            // owner,
            tags: Some(tags),
            target: Some(target),
            quantity: Some(quantity),
            data_root: data_root.to_base64_string()?,
            data_size,
            data,
            reward,
            signature: signature.to_base64_string()?,
        };

        debug!("trnsaction {:?}", &transaction);
        Ok(transaction)
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

        println!(
            "Posted transaction: https://arweave.net/{}",
            &transaction.id
        );
        Ok(())
    }
}
