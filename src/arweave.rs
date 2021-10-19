use async_trait::async_trait;
use base64;
use jsonwebkey::JsonWebKey;
use log::debug;
use num_bigint::{BigUint, ToBigInt};
use reqwest;
use ring::{
    digest::{Context, SHA256, SHA384},
    signature::{self, KeyPair, RsaKeyPair},
};
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncReadExt};
use url::Url;

type Error = Box<dyn std::error::Error>;

pub struct Provider {
    pub name: String,
    pub units: String,
    base: Url,
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
        base: Url::parse("https://arweave.net/")?,
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

#[async_trait]
pub trait Methods {
    fn wallet_address(&self) -> Result<String, Error>;
    async fn wallet_balance(&self, wallet_address: Option<String>) -> Result<BigUint, Error>;
    async fn price(&self, bytes: u32) -> Result<(BigUint, BigUint), Error>;
    async fn get_data(&self, id: &str) -> Result<(), Error>;
    fn verify_signature(&self, signature: &[u8], message: &[u8]) -> Result<(), Error>;
    async fn upload_file(&self, file_path: &str) -> Result<(), Error>;
}

#[derive(Serialize, Deserialize, Debug)]
struct TransactionArweave {
    format: u8,
    id: String,
    last_tx: String,
    owner: String,
    tags: Option<Vec<Tag>>,
    target: Option<String>,
    quantity: String,
    data_root: String,
    data: String,
    data_size: String,
    reward: String,
    signature: Option<String>,
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
    /// let calc = provider.wallet_address()?;
    /// let actual = String::from("7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg");
    /// assert_eq!(&calc, &actual);
    /// # Ok(())
    /// # }
    /// ```
    fn wallet_address(&self) -> Result<String, Error> {
        let mut context = Context::new(&SHA256);
        let modulus = self
            .keypair
            .public_key()
            .modulus()
            .big_endian_without_leading_zero();
        context.update(modulus);

        let wallet_address = base64::encode_config(context.finish(), base64::URL_SAFE_NO_PAD);
        Ok(wallet_address)
    }

    /// Returns the balance of the wallet.
    async fn wallet_balance(&self, wallet_address: Option<String>) -> Result<BigUint, Error> {
        let wallet_address = wallet_address.unwrap_or_else(|| self.wallet_address().unwrap());
        let url = self
            .base
            .join(&format!("wallet/{}/balance", &wallet_address))?;
        debug!("price url: {}", url);
        let winstons = reqwest::get(url).await?.json::<u64>().await?;
        Ok(BigUint::from(winstons))
    }

    /// Returns prices of uploading data to the network in winstons and usd per AR
    /// as a BigUint with two decimals.
    async fn price(&self, bytes: u32) -> Result<(BigUint, BigUint), Error> {
        let url = self.base.join("price/")?.join(&bytes.to_string())?;
        debug!("price url: {}", url);
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
    async fn get_data(&self, id: &str) -> Result<(), Error> {
        let url = self.base.join("tx/")?.join(id)?;
        debug!("get_data url: {}", url);
        let resp = reqwest::get(url)
            .await?
            .json::<TransactionArweave>()
            .await?;
        debug!("{:?}", resp);
        Ok(())
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
    /// let mut signature = vec![0; provider.keypair.public_modulus_len()];
    /// provider.keypair.sign(
    ///    &signature::RSA_PSS_SHA256,
    ///    &rng,
    ///    &message.as_bytes(),
    ///    &mut signature,
    /// )?;
    ///
    /// assert_eq!((), provider.verify_signature(&signature, &message.as_bytes())?);
    /// # Ok(())
    /// # }
    /// ```
    fn verify_signature(&self, signature: &[u8], message: &[u8]) -> Result<(), Error> {
        let public_key = signature::UnparsedPublicKey::new(
            &signature::RSA_PSS_2048_8192_SHA256,
            self.keypair.public_key().as_ref(),
        );
        public_key.verify(message, signature)?;
        Ok(())
    }

    /// Uploads a file to arweave.
    ///```
    /// use infer;
    /// # use metaplex_cli_lib::arweave::*;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = get_provider("tests/fixtures/0.json").await?;
    /// let file_path = &"tests/fixtures/0.json";
    /// provider.upload_file(&file_path);
    /// assert_eq!(file_path, &"tests/fixtures/0.json");
    /// # Ok(())
    /// # }
    /// ```
    async fn upload_file(&self, file_path: &str) -> Result<(), Error> {
        let mut file = File::open(file_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;
        let base64_string = base64::encode_config(buffer, base64::URL_SAFE_NO_PAD);
        let decoded = base64::decode_config(base64_string, base64::URL_SAFE_NO_PAD)?;
        println!("{}", String::from_utf8(decoded).unwrap());
        // assert_eq!("yo man", kind.mime_type());
        Ok(())
    }
}
