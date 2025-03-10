use jsonwebtoken::{DecodingKey, EncodingKey};
use std::sync::LazyLock;

static JWT_KEYS: LazyLock<JWTKeys> = LazyLock::new(|| {
    let secret = std::env::var("JWT_SECRET").expect("Missing JWT_SECRET env var");
    JWTKeys::new(secret.as_bytes())
});
struct JWTKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JWTKeys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}
