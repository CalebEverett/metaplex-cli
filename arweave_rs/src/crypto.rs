use async_trait::async_trait;
use jsonwebkey::JsonWebKey;
use log::debug;
use ring::{
    digest::{Context, SHA256, SHA384},
    rand,
    signature::{self, KeyPair, RsaKeyPair},
};
use tokio::{fs::File, io::AsyncReadExt};

type Error = Box<dyn std::error::Error>;

pub struct Provider {
    pub keypair: RsaKeyPair,
}

#[async_trait]
pub trait Methods {
    async fn from_keypair_path(keypair_path: &str) -> Result<Provider, Error>;
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error>;
    fn verify(&self, signature: &[u8], message: &[u8]) -> Result<(), Error>;
    #[allow(non_snake_case)]
    fn hash_SHA256(&self, message: &[u8]) -> Result<[u8; 32], Error>;
    #[allow(non_snake_case)]
    fn hash_SHA384(&self, message: &[u8]) -> Result<[u8; 48], Error>;
    #[allow(non_snake_case)]
    #[allow(non_snake_case)]
    fn hash_all_SHA256(&self, messages: Vec<&[u8]>) -> Result<[u8; 32], Error>;
    #[allow(non_snake_case)]
    fn hash_all_SHA384(&self, messages: Vec<&[u8]>) -> Result<[u8; 48], Error>;
    fn deep_hash(&self, data: Vec<&[u8]>) -> Result<[u8; 48], Error>;
    fn concat_u8_48(&self, left: [u8; 48], right: [u8; 48]) -> Result<[u8; 96], Error>;
}

#[async_trait]
impl Methods for Provider {
    async fn from_keypair_path(keypair_path: &str) -> Result<Provider, Error> {
        debug!("{:?}", keypair_path);
        let mut file = File::open(keypair_path).await?;
        let mut jwk_str = String::new();
        file.read_to_string(&mut jwk_str).await?;
        let jwk_parsed: JsonWebKey = jwk_str.parse().unwrap();
        Ok(Self {
            keypair: signature::RsaKeyPair::from_pkcs8(&jwk_parsed.key.as_ref().to_der())?,
        })
    }

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        let rng = rand::SystemRandom::new();
        let mut signature = vec![0; self.keypair.public_modulus_len()];
        self.keypair
            .sign(&signature::RSA_PSS_SHA256, &rng, message, &mut signature)?;
        Ok(signature)
    }

    /// Verifies that a message was signed by the public key of the Provider.key keypair.
    ///```
    /// # use ring::{signature, rand};
    /// # use arweave_rs::crypto::{Provider, Methods};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let crypto = Provider::from_keypair_path("tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json").await?;
    /// let message = String::from("hello, world");
    /// let rng = rand::SystemRandom::new();
    /// let signature = crypto.sign(&message.as_bytes())?;
    ///
    /// assert_eq!((), crypto.verify(&signature.as_ref(), &message.as_bytes())?);
    /// # Ok(())
    /// # }
    /// ```
    fn verify(&self, signature: &[u8], message: &[u8]) -> Result<(), Error> {
        let public_key = signature::UnparsedPublicKey::new(
            &signature::RSA_PSS_2048_8192_SHA256,
            self.keypair.public_key().as_ref(),
        );
        public_key.verify(message, signature)?;
        Ok(())
    }

    fn hash_SHA256(&self, message: &[u8]) -> Result<[u8; 32], Error> {
        let mut context = Context::new(&SHA256);
        context.update(message);
        let mut result: [u8; 32] = [0; 32];
        result.copy_from_slice(context.finish().as_ref());
        Ok(result)
    }

    fn hash_SHA384(&self, message: &[u8]) -> Result<[u8; 48], Error> {
        let mut context = Context::new(&SHA384);
        context.update(message);
        let mut result: [u8; 48] = [0; 48];
        result.copy_from_slice(context.finish().as_ref());
        Ok(result)
    }

    fn hash_all_SHA256(&self, messages: Vec<&[u8]>) -> Result<[u8; 32], Error> {
        let hash: Vec<u8> = messages
            .into_iter()
            .map(|m| self.hash_SHA256(m).unwrap())
            .into_iter()
            .flatten()
            .collect();
        let hash = self.hash_SHA256(&hash)?;
        Ok(hash)
    }

    fn hash_all_SHA384(&self, messages: Vec<&[u8]>) -> Result<[u8; 48], Error> {
        let hash: Vec<u8> = messages
            .into_iter()
            .map(|m| self.hash_SHA384(m).unwrap())
            .into_iter()
            .flatten()
            .collect();
        let hash = self.hash_SHA384(&hash)?;
        Ok(hash)
    }

    fn deep_hash(&self, data: Vec<&[u8]>) -> Result<[u8; 48], Error> {
        let list_tag = format!("list{}", data.len());
        let mut hash = self.hash_SHA384(list_tag.as_bytes())?;

        for blob in data.iter() {
            let blob_tag = format!("blob{}", blob.len());
            let blob_hash = self.hash_all_SHA384(vec![blob_tag.as_bytes(), blob])?;
            hash = self.hash_SHA384(&self.concat_u8_48(hash, blob_hash)?)?;
        }
        Ok(hash)
    }

    fn concat_u8_48(&self, left: [u8; 48], right: [u8; 48]) -> Result<[u8; 96], Error> {
        let mut iter = left.into_iter().chain(right);
        let result = [(); 96].map(|_| iter.next().unwrap());
        Ok(result)
    }
}

// Do one test without all of the default values specified and one with everything
// specified.

#[cfg(test)]
mod tests {
    use super::{Error, Methods, Provider};
    use crate::transaction::{Base64, ConvertUtf8, Tag, ToSlices, Transaction};
    use serde_json;
    use std::str::FromStr;
    use tokio::{fs::File, io::AsyncReadExt};

    #[tokio::test]
    async fn test_deep_hash() -> Result<(), Error> {
        let crypto = Provider::from_keypair_path(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        )
        .await?;

        let format = 2.to_string();

        let mut file = File::open("tests/fixtures/owner.json").await?;
        let mut data = String::new();
        file.read_to_string(&mut data).await?;

        let owner = serde_json::from_str::<Base64>(&data)?;
        let last_tx = Base64::from_str("LCwsLCwsLA")?;

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

        let transaction = Transaction {
            format: 2,
            owner: owner.clone(),
            reward: 42,
            last_tx,
            ..Default::default()
        };

        let hash = crypto.deep_hash(vec![
            &transaction.format.to_string().as_bytes(),
            &transaction.owner.0,
            &transaction.target.0,
            &transaction.quantity.to_string().as_bytes(),
            &transaction.reward.to_string().as_bytes(),
            &transaction.last_tx.0,
        ])?;

        println!("{:?}", hash);
        assert!(false);
        Ok(())
    }
}
