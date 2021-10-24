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

use crate::arweave2::merkle::HASH_SIZE;

#[async_trait]
pub trait Methods {
    async fn new(keypair_path: &str) -> Result<Provider, Error>;
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error>;
    fn verify(&self, signature: &[u8], message: &[u8]) -> Result<(), Error>;
    fn hash(&self, message: &[u8], algorithm: &str) -> Result<[u8; HASH_SIZE], Error>;
    fn hash_all(&self, messages: Vec<&[u8]>, algorithm: &str) -> Result<[u8; HASH_SIZE], Error>;
}

#[async_trait]
impl Methods for Provider {
    async fn new(keypair_path: &str) -> Result<Provider, Error> {
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

    fn verify(&self, signature: &[u8], message: &[u8]) -> Result<(), Error> {
        let public_key = signature::UnparsedPublicKey::new(
            &signature::RSA_PSS_2048_8192_SHA256,
            self.keypair.public_key().as_ref(),
        );
        public_key.verify(message, signature)?;
        Ok(())
    }

    fn hash(&self, message: &[u8], algorithm: &str) -> Result<[u8; HASH_SIZE], Error> {
        let algorithm = match algorithm {
            "SHA256" => &SHA256,
            "SHA384" => &SHA384,
            _ => unreachable!(),
        };
        let mut context = Context::new(algorithm);
        context.update(message);
        let mut result: [u8; HASH_SIZE] = Default::default();
        result.copy_from_slice(context.finish().as_ref());
        Ok(result)
    }
    fn hash_all(&self, messages: Vec<&[u8]>, algorithm: &str) -> Result<[u8; HASH_SIZE], Error> {
        let id: Vec<u8> = messages
            .into_iter()
            .map(|m| self.hash(m, algorithm).unwrap())
            .into_iter()
            .flatten()
            .collect();
        let id = self.hash(&id, algorithm)?;
        Ok(id)
    }
}
