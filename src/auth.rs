use jsonwebtoken::{DecodingKey, EncodingKey};
use std::env;
use std::sync::LazyLock;

pub static JWT_KEYS: LazyLock<JWTKeys> = LazyLock::new(|| {
    let secret = env::var("JWT_SECRET").expect("Missing JWT_SECRET env var");
    JWTKeys::new(secret.as_bytes())
});
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
