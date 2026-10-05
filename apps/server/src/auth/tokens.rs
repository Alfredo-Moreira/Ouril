//! Our own session tokens: a short-lived access JWT (HS256, `JWT_SECRET`) and an opaque,
//! rotating refresh token stored only as a SHA-256 hash.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const ISSUER: &str = "ouril";
pub const AUDIENCE: &str = "ouril-api";
/// Refresh tokens carry 256 bits of randomness.
const REFRESH_TOKEN_BYTES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessClaims {
    /// User ID.
    pub sub: String,
    /// Session ID.
    pub sid: String,
    pub iat: u64,
    pub exp: u64,
    pub iss: String,
    pub aud: String,
}

pub struct AccessTokens {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    pub ttl_secs: u32,
}

impl AccessTokens {
    pub fn new(secret: &str, ttl_secs: u32) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[ISSUER]);
        validation.set_audience(&[AUDIENCE]);
        validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub"]);
        validation.leeway = 30;
        AccessTokens {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            validation,
            ttl_secs,
        }
    }

    pub fn issue(
        &self,
        user_id: Uuid,
        session_id: Uuid,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = now_unix();
        let claims = AccessClaims {
            sub: user_id.to_string(),
            sid: session_id.to_string(),
            iat: now,
            exp: now + u64::from(self.ttl_secs),
            iss: ISSUER.into(),
            aud: AUDIENCE.into(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
    }

    /// Verify signature, algorithm, issuer, audience and expiry. Returns `(user_id, session_id)`.
    pub fn verify(&self, token: &str) -> Option<(Uuid, Uuid)> {
        let data = decode::<AccessClaims>(token, &self.decoding, &self.validation).ok()?;
        let user = Uuid::parse_str(&data.claims.sub).ok()?;
        let session = Uuid::parse_str(&data.claims.sid).ok()?;
        Some((user, session))
    }
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A fresh opaque refresh token (base64url, 43 chars).
pub fn new_refresh_token() -> String {
    let mut buf = [0u8; REFRESH_TOKEN_BYTES];
    getrandom::fill(&mut buf).expect("OS random number generator unavailable");
    URL_SAFE_NO_PAD.encode(buf)
}

/// What we store: SHA-256 of the token. Tokens are high-entropy, so no salt/KDF is needed.
pub fn hash_refresh_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Cheap shape check before touching the database.
pub fn looks_like_refresh_token(token: &str) -> bool {
    token.len() == 43
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-test-secret-test-secret!!";

    #[test]
    fn access_token_round_trip() {
        let t = AccessTokens::new(SECRET, 900);
        let (u, s) = (Uuid::now_v7(), Uuid::now_v7());
        let jwt = t.issue(u, s).unwrap();
        assert_eq!(t.verify(&jwt), Some((u, s)));
    }

    #[test]
    fn rejects_wrong_secret_and_garbage() {
        let a = AccessTokens::new(SECRET, 900);
        let b = AccessTokens::new("another-secret-another-secret-another", 900);
        let jwt = a.issue(Uuid::now_v7(), Uuid::now_v7()).unwrap();
        assert!(b.verify(&jwt).is_none());
        assert!(a.verify("not.a.jwt").is_none());
    }

    #[test]
    fn rejects_expired() {
        let t = AccessTokens::new(SECRET, 900);
        let claims = AccessClaims {
            sub: Uuid::now_v7().to_string(),
            sid: Uuid::now_v7().to_string(),
            iat: 1,
            exp: 2,
            iss: ISSUER.into(),
            aud: AUDIENCE.into(),
        };
        let jwt = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();
        assert!(t.verify(&jwt).is_none());
    }

    #[test]
    fn refresh_tokens_are_unique_and_hashed() {
        let a = new_refresh_token();
        let b = new_refresh_token();
        assert_ne!(a, b);
        assert!(looks_like_refresh_token(&a));
        assert_eq!(hash_refresh_token(&a).len(), 32);
        assert_ne!(hash_refresh_token(&a), hash_refresh_token(&b));
    }

    fn signed(claims: &AccessClaims, secret: &str, alg: Algorithm) -> String {
        encode(
            &Header::new(alg),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    fn valid_claims() -> AccessClaims {
        let now = now_unix();
        AccessClaims {
            sub: Uuid::now_v7().to_string(),
            sid: Uuid::now_v7().to_string(),
            iat: now,
            exp: now + 600,
            iss: ISSUER.into(),
            aud: AUDIENCE.into(),
        }
    }

    #[test]
    fn issued_claims_are_complete() {
        let t = AccessTokens::new(SECRET, 900);
        let (u, s) = (Uuid::now_v7(), Uuid::now_v7());
        let jwt = t.issue(u, s).unwrap();
        let mut v = Validation::new(Algorithm::HS256);
        v.set_audience(&[AUDIENCE]);
        let c = decode::<AccessClaims>(&jwt, &DecodingKey::from_secret(SECRET.as_bytes()), &v)
            .unwrap()
            .claims;
        assert_eq!(c.sub, u.to_string());
        assert_eq!(c.sid, s.to_string());
        assert_eq!(c.iss, ISSUER);
        assert_eq!(c.aud, AUDIENCE);
        assert_eq!(c.exp - c.iat, 900);
    }

    #[test]
    fn rejects_wrong_issuer_audience_algorithm_and_ids() {
        let t = AccessTokens::new(SECRET, 900);
        assert!(t
            .verify(&signed(&valid_claims(), SECRET, Algorithm::HS256))
            .is_some());
        let cases = [
            AccessClaims {
                iss: "other".into(),
                ..valid_claims()
            },
            AccessClaims {
                aud: "other".into(),
                ..valid_claims()
            },
            AccessClaims {
                sub: "not-a-uuid".into(),
                ..valid_claims()
            },
            AccessClaims {
                sid: "not-a-uuid".into(),
                ..valid_claims()
            },
        ];
        for (i, c) in cases.iter().enumerate() {
            assert!(
                t.verify(&signed(c, SECRET, Algorithm::HS256)).is_none(),
                "case {i}"
            );
        }
        // Same secret, different HMAC algorithm: refused.
        assert!(t
            .verify(&signed(&valid_claims(), SECRET, Algorithm::HS512))
            .is_none());
        // Unsigned token ("alg": "none").
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let body = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&valid_claims()).unwrap());
        assert!(t.verify(&format!("{header}.{body}.")).is_none());
    }

    #[test]
    fn expiry_leeway_is_small() {
        let t = AccessTokens::new(SECRET, 900);
        let now = now_unix();
        let just_expired = AccessClaims {
            exp: now - 5,
            ..valid_claims()
        };
        assert!(
            t.verify(&signed(&just_expired, SECRET, Algorithm::HS256))
                .is_some(),
            "30 s leeway"
        );
        let long_expired = AccessClaims {
            exp: now - 120,
            ..valid_claims()
        };
        assert!(t
            .verify(&signed(&long_expired, SECRET, Algorithm::HS256))
            .is_none());
    }

    #[test]
    fn refresh_token_shape_and_hash() {
        let a = new_refresh_token();
        assert_eq!(a.len(), 43);
        // Deterministic hash, so lookups work.
        assert_eq!(hash_refresh_token(&a), hash_refresh_token(&a));
        assert_eq!(
            hash_refresh_token("abc"),
            Sha256::digest(b"abc").to_vec(),
            "plain SHA-256"
        );
        for bad in [
            "",
            "short",
            &"a".repeat(42),
            &"a".repeat(44),
            &"+".repeat(43),
            &"/".repeat(43),
            &"=".repeat(43),
        ] {
            assert!(!looks_like_refresh_token(bad), "{bad:?}");
        }
        assert!(looks_like_refresh_token(&"-_aZ09".repeat(8)[..43]));
    }
}
