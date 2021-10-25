use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_aux::field_attributes::deserialize_number_from_string;
use std::str::FromStr;

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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Transaction {
    pub format: u8,
    pub id: Base64,
    pub last_tx: Base64,
    pub owner: Base64,
    pub tags: Vec<Tag>,
    pub target: Base64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub quantity: u64,
    pub data_root: Base64,
    pub data: Base64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub data_size: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub reward: u64,
    pub signature: Base64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tag {
    pub name: Base64,
    pub value: Base64,
}

pub trait ToSlices<'a, T> {
    fn to_slices(&'a self) -> Result<Vec<Vec<&'a [u8]>>, Error>;
}

impl<'a> ToSlices<'a, Vec<Tag>> for Vec<Tag> {
    fn to_slices(&'a self) -> Result<Vec<Vec<&'a [u8]>>, Error> {
        let result = self
            .iter()
            .map(|t| vec![&t.name.0[..], &t.value.0[..]])
            .collect();
        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct Base64(pub Vec<u8>);
impl Serialize for Base64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format!("{}", &self))
    }
}

impl Default for Base64 {
    fn default() -> Self {
        Base64(vec![])
    }
}

/// Converts a base64url encoded string to a Base64 struct.
impl FromStr for Base64 {
    type Err = base64::DecodeError;
    fn from_str(str: &str) -> Result<Self, Self::Err> {
        let result = base64::decode_config(str, base64::URL_SAFE_NO_PAD)?;
        Ok(Self(result))
    }
}

/// Handles conversion of unencoded strings through to base64url and back to bytes.
pub trait ConvertUtf8<T> {
    fn from_utf8_str(str: &str) -> Result<T, Error>;
    fn to_utf8_string(&self) -> Result<String, Error>;
}

impl ConvertUtf8<Base64> for Base64 {
    fn from_utf8_str(str: &str) -> Result<Self, Error> {
        let result = base64::encode_config(str, base64::URL_SAFE_NO_PAD);
        Ok(Self(result.as_bytes().to_vec()))
    }
    fn to_utf8_string(&self) -> Result<String, Error> {
        let bytes_vec = base64::decode_config(&self.0, base64::URL_SAFE_NO_PAD)?;
        let string = String::from_utf8(bytes_vec)?;
        Ok(string)
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
                    .map_err(|_| de::Error::custom("failed to decode base64 string"))
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
#[cfg(test)]
mod tests {
    use super::{Base64, ConvertUtf8, Error, Tag, ToSlices};
    // use serde::{self, de, Deserialize, Deserializer, Serialize, Serializer};
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn test_deserialize_base64() -> Result<(), Error> {
        let base_64 = Base64(vec![44; 7]);
        assert_eq!(base_64.0, vec![44; 7]);
        assert_eq!(format!("{}", base_64), "LCwsLCwsLA");

        let base_64: Base64 = serde_json::from_str("\"LCwsLCwsLA\"")?;
        assert_eq!(base_64.0, vec![44; 7]);
        assert_eq!(format!("{}", base_64), "LCwsLCwsLA");
        Ok(())
    }

    #[test]
    fn test_base64_convert_utf8() -> Result<(), Error> {
        let string_b64 = Base64::from_utf8_str("gnarlycharcters[-093210342/~==%$")?;
        assert_eq!(
            "gnarlycharcters[-093210342/~==%$".to_string(),
            string_b64.to_utf8_string()?
        );
        let string_b64 = Base64::from_utf8_str("foo")?;
        assert_eq!("foo".to_string(), string_b64.to_utf8_string()?);
        assert_eq!("Wm05dg".to_string(), string_b64.to_string());
        Ok(())
    }

    #[test]
    fn test_transaction_slices() -> Result<(), Error> {
        let tags = Vec::<Tag>::new();
        assert_eq!(tags.to_slices()?, Vec::<Vec<&[u8]>>::new());

        let tags = vec![
            Tag {
                name: Base64::from_utf8_str("Content-Type")?,
                value: Base64::from_utf8_str("text/html")?,
            },
            Tag {
                name: Base64::from_utf8_str("key2")?,
                value: Base64::from_utf8_str("value2")?,
            },
        ];

        assert_eq!(
            "UTI5dWRHVnVkQzFVZVhCbA".to_string(),
            tags[0].name.to_string()
        );

        assert_eq!("Content-Type".to_string(), tags[0].name.to_utf8_string()?);
        let tag_slices = tags.to_slices()?;
        println!("{:?}", tag_slices);
        assert_eq!(tag_slices.len(), 2);
        tag_slices.iter().for_each(|f| assert_eq!(f.len(), 2));
        assert_eq!(
            tag_slices[0][0],
            &[81, 50, 57, 117, 100, 71, 86, 117, 100, 67, 49, 85, 101, 88, 66, 108][..]
        );
        assert_eq!(
            tag_slices[0][1],
            &[100, 71, 86, 52, 100, 67, 57, 111, 100, 71, 49, 115][..]
        );
        assert_eq!(tag_slices[1][0], &[97, 50, 86, 53, 77, 103][..]);
        assert_eq!(tag_slices[1][1], &[100, 109, 70, 115, 100, 87, 85, 121][..]);
        Ok(())
    }
}
