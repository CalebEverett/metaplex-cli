use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_aux::field_attributes::deserialize_number_from_string;
use std::str;

use async_trait::async_trait;

use infer;
use log::debug;
use num_bigint::BigUint;
use reqwest::{
    self,
    header::{ACCEPT, CONTENT_TYPE},
};

use tokio::{fs::File, io::AsyncReadExt};
use url::Url;

type Error = Box<dyn std::error::Error>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    format: u8,
    pub id: Base64,
    last_tx: Base64,
    owner: Base64,
    tags: Option<Vec<Tag>>,
    target: Option<Base64>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    quantity: u64,
    data_root: Option<Base64>,
    data: Option<Base64>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    data_size: u64,
    reward: String,
    signature: Base64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Tag {
    name: String,
    value: String,
}

#[derive(Debug)]
pub struct Base64(Vec<u8>);
impl Serialize for Base64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format!("{}", &self))
    }
}

impl<'de> Deserialize<'de> for Base64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Vis;
        impl serde::de::Visitor<'_> for Vis {
            type Value = Base64;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a base64 string")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                base64::decode_config(v, base64::URL_SAFE_NO_PAD)
                    .map(Base64)
                    .map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_str(Vis)
    }
}

impl std::fmt::Display for Base64 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let string = &base64::display::Base64Display::with_config(&self.0, base64::URL_SAFE_NO_PAD);
        write!(f, "{}", string)
    }
}

async fn transaction_from_filepath(file_path: &str) -> Result<(), Error> {
    // Read file to buffer
    let mut file = File::open(file_path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;

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
    Ok(())
}
