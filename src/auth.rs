use jsonwebtoken::{DecodingKey, EncodingKey};
use once_cell::sync::Lazy;
use std::env;
use tracing::{error, info};

pub struct JWTKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl JWTKeys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

pub static JWT_KEYS: Lazy<JWTKeys> = Lazy::new(|| {
    info!("Initializing JWT_KEYS...");
    let secret = match env::var("JWT_SECRET") {
        Ok(val) => {
            info!("JWT_SECRET found: {}", val);
            val
        }
        Err(e) => {
            error!("Missing JWT_SECRET env var: {}", e);
            panic!("Missing JWT_SECRET env var: {}", e);
        }
    };
    let secret_bytes = secret.as_bytes();
    info!("JWT_SECRET as bytes: {:?}", secret_bytes);
    JWTKeys::new(secret_bytes)
});
