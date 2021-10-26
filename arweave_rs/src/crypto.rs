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
use crate::transaction::{Base64, Tag, ToSlices};

pub struct Provider {
    pub keypair: RsaKeyPair,
}

#[async_trait]
pub trait Methods {
    async fn from_keypair_path(keypair_path: &str) -> Result<Provider, Error>;
    fn keypair_modulus(&self) -> Result<Base64, Error>;
    fn wallet_address(&self) -> Result<Base64, Error>;
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
    fn deep_hash_list(
        &self,
        data_len: usize,
        data: Vec<&[u8]>,
        hash: Option<[u8; 48]>,
    ) -> Result<[u8; 48], Error>;
    fn deep_hash_tags(&self, tags: Vec<Tag>) -> Result<[u8; 48], Error>;
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
    fn keypair_modulus(&self) -> Result<Base64, Error> {
        let modulus = self
            .keypair
            .public_key()
            .modulus()
            .big_endian_without_leading_zero();
        Ok(Base64(modulus.to_vec()))
    }
    /// Calculates the wallet address of the provided keypair according to [addressing](https://docs.arweave.org/developers/server/http-api#addressing)
    /// in documentation.
    ///```
    /// # use arweave_rs::crypto::Methods as CryptoMethods;
    /// # use arweave_rs::{Arweave, Methods as ArweaveMethods};
    /// # use ring::{signature, rand};
    /// # use std::fmt::Display;
    /// #
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let arweave = Arweave::from_keypair_path("tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json").await?;
    /// let calc = arweave.crypto.wallet_address()?;
    /// let actual = String::from("7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg");
    /// assert_eq!(&calc.to_string(), &actual);
    /// # Ok(())
    /// # }
    /// ```
    fn wallet_address(&self) -> Result<Base64, Error> {
        let mut context = Context::new(&SHA256);
        context.update(&self.keypair_modulus()?.0[..]);
        let wallet_address = Base64(context.finish().as_ref().to_vec());
        Ok(wallet_address)
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

    fn deep_hash_list(
        &self,
        data_len: usize,
        data: Vec<&[u8]>,
        hash: Option<[u8; 48]>,
    ) -> Result<[u8; 48], Error> {
        let mut hash = if let Some(hash) = hash {
            hash
        } else {
            let list_tag = format!("list{}", data_len);
            self.hash_SHA384(list_tag.as_bytes())?
        };

        for blob in data.iter() {
            let blob_tag = format!("blob{}", blob.len());
            let blob_hash = self.hash_all_SHA384(vec![blob_tag.as_bytes(), blob])?;
            hash = self.hash_SHA384(&self.concat_u8_48(hash, blob_hash)?)?;
        }
        Ok(hash)
    }

    fn deep_hash_tags(&self, tags: Vec<Tag>) -> Result<[u8; 48], Error> {
        let list_tag = format!("list{}", tags.len());
        let mut hash = self.hash_SHA384(list_tag.as_bytes())?;

        for tag_slice in tags.to_slices()?.into_iter() {
            let tag_slice_hash = self.deep_hash_list(tag_slice.len(), tag_slice, None)?;
            hash = self.hash_SHA384(&self.concat_u8_48(hash, tag_slice_hash)?)?;
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
    use crate::{
        crypto::Methods as CryptoMethods,
        transaction::{Base64, ConvertUtf8, Tag},
        Arweave, Error, Methods as ArewaveMethods,
    };
    use std::str::FromStr;

    #[tokio::test]
    async fn test_deep_hash() -> Result<(), Error> {
        let arweave = Arweave::from_keypair_path(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        )
        .await?;

        let file_paths = ["0.png", "1mb.bin"];
        let hashes: [[u8; 48]; 2] = [
            [
                247, 203, 170, 29, 75, 110, 63, 222, 164, 171, 56, 33, 222, 227, 81, 22, 101, 250,
                103, 139, 83, 102, 39, 10, 253, 84, 189, 84, 155, 223, 91, 0, 179, 47, 152, 105,
                206, 78, 57, 73, 254, 1, 235, 80, 139, 125, 180, 122,
            ],
            [
                93, 159, 217, 53, 116, 202, 121, 98, 75, 149, 24, 99, 56, 77, 154, 34, 195, 141,
                56, 137, 228, 254, 88, 98, 162, 101, 10, 122, 114, 72, 106, 150, 98, 173, 7, 103,
                31, 99, 156, 206, 75, 0, 215, 65, 97, 58, 158, 186,
            ],
        ];

        for (file_path, correct_hash) in file_paths.iter().zip(hashes) {
            let mut transaction = arweave
                .create_transaction_from_file_path(&format!("tests/fixtures/{}", file_path))
                .await?;

            transaction.last_tx = Base64::from_str("LCwsLCwsLA")?;

            transaction.tags = vec![
                Tag {
                    name: Base64::from_utf8_str("Content-Type")?,
                    value: Base64::from_utf8_str("text/html")?,
                },
                Tag {
                    name: Base64::from_utf8_str("key2")?,
                    value: Base64::from_utf8_str("value2")?,
                },
            ];

            transaction.target = arweave.crypto.wallet_address()?;

            transaction.reward = 42;

            let pre_tag_hash = arweave.crypto.deep_hash_list(
                9,
                vec![
                    &transaction.format.to_string().as_bytes(),
                    &transaction.owner.0,
                    &transaction.target.0,
                    &transaction.quantity.to_string().as_bytes(),
                    &transaction.reward.to_string().as_bytes(),
                    &transaction.last_tx.0,
                ],
                None,
            )?;

            let tag_hash = arweave.crypto.deep_hash_tags(transaction.tags)?;
            let post_tag_hash = arweave
                .crypto
                .hash_SHA384(&arweave.crypto.concat_u8_48(pre_tag_hash, tag_hash)?)?;
            let hash = arweave.crypto.deep_hash_list(
                0,
                vec![
                    &transaction.data_size.to_string().as_bytes(),
                    &transaction.data_root.0,
                ],
                Some(post_tag_hash),
            )?;
            assert_eq!(hash, correct_hash);
        }
        Ok(())
    }
}
