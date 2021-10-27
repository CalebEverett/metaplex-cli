use crate::merkle::{Node, Proof};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

type Error = Box<dyn std::error::Error>;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Transaction {
    pub format: u8,
    pub id: Base64,
    pub last_tx: Base64,
    pub owner: Base64,
    pub tags: Vec<Tag>,
    pub target: Base64,
    #[serde(with = "stringify")]
    pub quantity: u64,
    pub data_root: Base64,
    pub data: Base64,
    #[serde(with = "stringify")]
    pub data_size: u64,
    #[serde(with = "stringify")]
    pub reward: u64,
    pub signature: Base64,
    #[serde(skip)]
    pub chunks: Vec<Node>,
    #[serde(skip)]
    pub proofs: Vec<Proof>,
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

pub mod stringify {
    use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        String::deserialize(deserializer)?
            .parse::<T>()
            .map_err(|e| D::Error::custom(format!("{}", e)))
    }

    pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: std::fmt::Display,
    {
        format!("{}", value).serialize(serializer)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Base64(pub Vec<u8>);

impl Default for Base64 {
    fn default() -> Self {
        Base64(vec![])
    }
}

impl std::fmt::Display for Base64 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let string = &base64::display::Base64Display::with_config(&self.0, base64::URL_SAFE_NO_PAD);
        write!(f, "{}", string)
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
        let enc_string = base64::encode_config(str.as_bytes(), base64::URL_SAFE_NO_PAD);
        let dec_bytes = base64::decode_config(enc_string, base64::URL_SAFE_NO_PAD)?;
        Ok(Self(dec_bytes))
    }
    fn to_utf8_string(&self) -> Result<String, Error> {
        let enc_string = base64::encode_config(&self.0, base64::URL_SAFE_NO_PAD);
        let dec_bytes = base64::decode_config(enc_string, base64::URL_SAFE_NO_PAD)?;
        Ok(String::from_utf8(dec_bytes)?)
    }
}

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
                    .map_err(|_| de::Error::custom("failed to decode base64 string"))
            }
        }
        deserializer.deserialize_str(Vis)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tag {
    pub name: Base64,
    pub value: Base64,
}

pub trait FromStrs<T> {
    fn from_utf8_strs(name: &str, value: &str) -> Result<T, Error>;
}

impl FromStrs<Tag> for Tag {
    fn from_utf8_strs(name: &str, value: &str) -> Result<Self, Error> {
        let b64_name = Base64::from_utf8_str(name)?;
        let b64_value = Base64::from_utf8_str(value)?;

        Ok(Self {
            name: b64_name,
            value: b64_value,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::transaction::FromStrs;

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
        let foo_b64 = Base64::from_utf8_str("foo")?;
        assert_eq!(foo_b64.0, vec![102, 111, 111]);

        let foo_b64 = Base64(vec![102, 111, 111]);
        assert_eq!(foo_b64.to_utf8_string()?, "foo".to_string());
        Ok(())
    }

    #[test]
    fn test_base64_convert_string() -> Result<(), Error> {
        let foo_b64 = Base64::from_str("LCwsLCwsLA")?;
        assert_eq!(foo_b64.0, vec![44; 7]);

        let foo_b64 = Base64(vec![44; 7]);
        assert_eq!(foo_b64.to_string(), "LCwsLCwsLA".to_string());
        Ok(())
    }

    #[test]
    fn test_transaction_slices() -> Result<(), Error> {
        let tags = Vec::<Tag>::new();
        assert_eq!(tags.to_slices()?, Vec::<Vec<&[u8]>>::new());

        let tags = vec![
            Tag::from_utf8_strs("Content-Type", "text/html")?,
            Tag::from_utf8_strs("key2", "value2")?,
        ];

        assert_eq!("Content-Type".to_string(), tags[0].name.to_utf8_string()?);
        assert_eq!("Q29udGVudC1UeXBl".to_string(), tags[0].name.to_string());

        let tag_slices = tags.to_slices()?;
        assert_eq!(tag_slices.len(), 2);
        tag_slices.iter().for_each(|f| assert_eq!(f.len(), 2));
        assert_eq!(
            tag_slices[0][0],
            &[67, 111, 110, 116, 101, 110, 116, 45, 84, 121, 112, 101][..]
        );
        assert_eq!(
            tag_slices[0][1],
            &[116, 101, 120, 116, 47, 104, 116, 109, 108][..]
        );
        assert_eq!(tag_slices[1][0], &[107, 101, 121, 50][..]);
        assert_eq!(tag_slices[1][1], &[118, 97, 108, 117, 101, 50][..]);
        Ok(())
    }
}
