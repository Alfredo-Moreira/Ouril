//! OpenID Connect ID-token verification for Google and Apple (ADR 0009).
//!
//! **Placeholder status:** this is wired only when the provider is configured
//! (`GOOGLE_CLIENT_ID`, `APPLE_*`). Until then the endpoints answer 501 `not_configured` and
//! nothing here runs, so no request ever leaves the server. It has not been exercised against
//! real Google/Apple tokens yet: test with real client IDs before enabling in production.
//!
//! Checks: RS256 signature against the provider's JWKS (cached, refetched on unknown `kid`
//! at most once a minute), issuer, audience, expiry (+60 s leeway) and nonce.

use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

const JWKS_TTL: Duration = Duration::from_secs(3600);
const JWKS_MIN_REFETCH: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedIdToken {
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    /// Apple "Hide My Email" relay address.
    pub is_private_email: bool,
    /// Google only (Apple sends the name in the request body on first sign-in).
    pub name: Option<String>,
    /// The audience the token was issued for (Apple: Services ID or bundle ID). Apple's token
    /// exchange and revocation must use the same client ID.
    pub audience: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// The token is malformed, expired, for another audience, or the signature/nonce is wrong.
    Invalid(&'static str),
    /// JWKS couldn't be fetched (provider down / no network).
    KeysUnavailable,
}

/// Google sends booleans; Apple sends `"true"`/`"false"` strings.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LooseBool {
    Bool(bool),
    Str(String),
}

impl LooseBool {
    fn get(&self) -> bool {
        match self {
            LooseBool::Bool(b) => *b,
            LooseBool::Str(s) => s.eq_ignore_ascii_case("true"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    sub: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    email_verified: Option<LooseBool>,
    #[serde(default)]
    is_private_email: Option<LooseBool>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    aud: Option<serde_json::Value>,
}

pub struct OidcVerifier {
    pub provider: &'static str,
    issuers: Vec<String>,
    audiences: Vec<String>,
    jwks_uri: &'static str,
    http: reqwest::Client,
    cache: RwLock<Option<(Instant, JwkSet)>>,
}

impl OidcVerifier {
    pub fn google(client_id: String) -> Self {
        Self::new(
            "google",
            vec![
                "https://accounts.google.com".into(),
                "accounts.google.com".into(),
            ],
            vec![client_id],
            "https://www.googleapis.com/oauth2/v3/certs",
        )
    }

    /// Web tokens use the Services ID as audience; native iOS tokens use the app's bundle ID
    /// (`APPLE_BUNDLE_ID`), accepted too when set.
    pub fn apple(service_id: String, bundle_id: Option<String>) -> Self {
        let mut audiences = vec![service_id];
        audiences.extend(bundle_id);
        Self::new(
            "apple",
            vec!["https://appleid.apple.com".into()],
            audiences,
            "https://appleid.apple.com/auth/keys",
        )
    }

    fn new(
        provider: &'static str,
        issuers: Vec<String>,
        audiences: Vec<String>,
        jwks_uri: &'static str,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("HTTP client");
        OidcVerifier {
            provider,
            issuers,
            audiences,
            jwks_uri,
            http,
            cache: RwLock::new(None),
        }
    }

    pub async fn verify(
        &self,
        id_token: &str,
        nonce: &str,
    ) -> Result<VerifiedIdToken, VerifyError> {
        if id_token.len() > 8192 || nonce.is_empty() || nonce.len() > 256 {
            return Err(VerifyError::Invalid("malformed token or nonce"));
        }
        let header =
            decode_header(id_token).map_err(|_| VerifyError::Invalid("malformed token"))?;
        if header.alg != Algorithm::RS256 {
            return Err(VerifyError::Invalid("unexpected signing algorithm"));
        }
        let kid = header
            .kid
            .ok_or(VerifyError::Invalid("token has no key id"))?;
        let key = self.key_for(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&self.issuers);
        validation.set_audience(&self.audiences);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.leeway = 60;
        let data = decode::<IdTokenClaims>(id_token, &key, &validation).map_err(|_| {
            VerifyError::Invalid("signature, issuer, audience or expiry check failed")
        })?;
        let claims = data.claims;

        // Google echoes the raw nonce. Apple apps usually send SHA-256(nonce) to Apple and the
        // raw nonce to us, so accept either form.
        let token_nonce = claims
            .nonce
            .as_deref()
            .ok_or(VerifyError::Invalid("token has no nonce"))?;
        let hashed = hex(&Sha256::digest(nonce.as_bytes()));
        if token_nonce != nonce && token_nonce != hashed {
            return Err(VerifyError::Invalid("nonce mismatch"));
        }
        if claims.sub.is_empty() || claims.sub.len() > 255 {
            return Err(VerifyError::Invalid("invalid subject"));
        }
        // `aud` is a string or an array; validation already checked it is one of ours.
        let audience = match &claims.aud {
            Some(serde_json::Value::String(a)) => Some(a.clone()),
            Some(serde_json::Value::Array(v)) => v
                .iter()
                .filter_map(|a| a.as_str())
                .find(|a| self.audiences.iter().any(|ours| ours == a))
                .map(str::to_string),
            _ => None,
        };
        Ok(VerifiedIdToken {
            subject: claims.sub,
            email: claims.email,
            email_verified: claims.email_verified.is_some_and(|b| b.get()),
            is_private_email: claims.is_private_email.is_some_and(|b| b.get()),
            name: claims.name,
            audience,
        })
    }

    async fn key_for(&self, kid: &str) -> Result<DecodingKey, VerifyError> {
        if let Some(found) = cached_key(self.cache.read().await.as_ref(), kid) {
            return found;
        }
        let mut cache = self.cache.write().await;
        // Another request may have refreshed the set while this one waited for the lock:
        // reuse it instead of fetching again.
        if let Some(found) = cached_key(cache.as_ref(), kid) {
            return found;
        }
        let set = self.fetch_jwks().await?;
        let key = set
            .find(kid)
            .map(DecodingKey::from_jwk)
            .transpose()
            .map_err(|_| VerifyError::Invalid("unusable key"))?;
        *cache = Some((Instant::now(), set));
        key.ok_or(VerifyError::Invalid("unknown key id"))
    }

    async fn fetch_jwks(&self) -> Result<JwkSet, VerifyError> {
        let res = self.http.get(self.jwks_uri).send().await.map_err(|e| {
            tracing::error!(provider = self.provider, error = %e, "JWKS fetch failed");
            VerifyError::KeysUnavailable
        })?;
        res.error_for_status()
            .map_err(|_| VerifyError::KeysUnavailable)?
            .json::<JwkSet>()
            .await
            .map_err(|_| VerifyError::KeysUnavailable)
    }
}

/// Answer from the cached JWKS when it can: `Some(Ok)` if the key is cached and fresh,
/// `Some(Err("unknown key id"))` if the set was fetched less than [`JWKS_MIN_REFETCH`] ago and
/// still lacks the key, `None` when a fetch is needed.
fn cached_key(
    cache: Option<&(Instant, JwkSet)>,
    kid: &str,
) -> Option<Result<DecodingKey, VerifyError>> {
    let (fetched, set) = cache?;
    let age = fetched.elapsed();
    if age >= JWKS_TTL {
        return None;
    }
    if let Some(jwk) = set.find(kid) {
        return Some(DecodingKey::from_jwk(jwk).map_err(|_| VerifyError::Invalid("unusable key")));
    }
    if age < JWKS_MIN_REFETCH {
        return Some(Err(VerifyError::Invalid("unknown key id")));
    }
    None
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    //! Offline tests: the JWKS cache is pre-filled with a test-only RSA key
    //! (`tests/fixtures/oidc_test_only_rsa.pem`, generated for these tests, never used
    //! anywhere else), so no request ever reaches Google or Apple.

    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::{json, Value};

    const TEST_KEY_PEM: &str = include_str!("../../tests/fixtures/oidc_test_only_rsa.pem");
    const TEST_KEY_N: &str = include_str!("../../tests/fixtures/oidc_test_only_rsa.n");
    const KID: &str = "test-kid";
    const CLIENT_ID: &str = "test-client.apps.googleusercontent.com";

    async fn google_with_test_key() -> OidcVerifier {
        let v = OidcVerifier::google(CLIENT_ID.into());
        let set: JwkSet = serde_json::from_value(json!({
            "keys": [{
                "kty": "RSA", "kid": KID, "alg": "RS256", "use": "sig",
                "n": TEST_KEY_N.trim(), "e": "AQAB"
            }]
        }))
        .unwrap();
        *v.cache.write().await = Some((Instant::now(), set));
        v
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn claims() -> Value {
        json!({
            "iss": "https://accounts.google.com",
            "aud": CLIENT_ID,
            "sub": "google-subject-1",
            "exp": now() + 600,
            "iat": now(),
            "nonce": "raw-nonce",
            "email": "ana@example.com",
            "email_verified": true,
            "name": "Ana Lopes"
        })
    }

    fn sign(claims: &Value, kid: Option<&str>) -> String {
        let mut h = Header::new(Algorithm::RS256);
        h.kid = kid.map(str::to_string);
        let key = EncodingKey::from_rsa_pem(TEST_KEY_PEM.as_bytes()).expect("test key");
        encode(&h, claims, &key).unwrap()
    }

    #[tokio::test]
    async fn accepts_a_valid_token() {
        let v = google_with_test_key().await;
        let t = v
            .verify(&sign(&claims(), Some(KID)), "raw-nonce")
            .await
            .unwrap();
        assert_eq!(t.subject, "google-subject-1");
        assert_eq!(t.email.as_deref(), Some("ana@example.com"));
        assert!(t.email_verified);
        assert!(!t.is_private_email);
        assert_eq!(t.name.as_deref(), Some("Ana Lopes"));
        assert_eq!(t.audience.as_deref(), Some(CLIENT_ID));
        // The issuer without scheme is also Google's.
        let mut c = claims();
        c["iss"] = json!("accounts.google.com");
        assert!(v.verify(&sign(&c, Some(KID)), "raw-nonce").await.is_ok());
    }

    #[tokio::test]
    async fn accepts_hashed_nonce_and_string_booleans() {
        let v = google_with_test_key().await;
        let mut c = claims();
        c["nonce"] = json!(hex(&Sha256::digest(b"raw-nonce")));
        c["email_verified"] = json!("true");
        c["is_private_email"] = json!("TRUE");
        let t = v.verify(&sign(&c, Some(KID)), "raw-nonce").await.unwrap();
        assert!(t.email_verified);
        assert!(t.is_private_email);
    }

    #[tokio::test]
    async fn rejects_bad_claims() {
        let v = google_with_test_key().await;
        type Tamper = fn(&mut Value);
        let cases: [(&str, Tamper); 9] = [
            ("wrong audience", |c| c["aud"] = json!("someone-else")),
            ("wrong issuer", |c| c["iss"] = json!("https://evil.example")),
            ("expired", |c| c["exp"] = json!(now() - 3600)),
            ("nonce mismatch", |c| c["nonce"] = json!("other-nonce")),
            ("no nonce", |c| {
                c.as_object_mut().unwrap().remove("nonce");
            }),
            ("no sub", |c| {
                c.as_object_mut().unwrap().remove("sub");
            }),
            ("empty sub", |c| c["sub"] = json!("")),
            ("long sub", |c| c["sub"] = json!("s".repeat(256))),
            ("no exp", |c| {
                c.as_object_mut().unwrap().remove("exp");
            }),
        ];
        for (name, tamper) in cases {
            let mut c = claims();
            tamper(&mut c);
            let res = v.verify(&sign(&c, Some(KID)), "raw-nonce").await;
            assert!(
                matches!(res, Err(VerifyError::Invalid(_))),
                "{name}: {res:?}"
            );
        }
    }

    #[tokio::test]
    async fn rejects_bad_tokens_before_fetching_keys() {
        // Fresh verifier with an empty cache: these must fail without touching the network.
        let v = OidcVerifier::google(CLIENT_ID.into());
        let invalid =
            |r: Result<VerifiedIdToken, VerifyError>| matches!(r, Err(VerifyError::Invalid(_)));
        assert!(invalid(v.verify("not-a-jwt", "n").await));
        assert!(
            invalid(v.verify(&sign(&claims(), Some(KID)), "").await),
            "empty nonce"
        );
        assert!(invalid(
            v.verify(&sign(&claims(), Some(KID)), &"n".repeat(257))
                .await
        ));
        assert!(
            invalid(v.verify(&"a".repeat(8193), "n").await),
            "oversized token"
        );
        assert!(
            invalid(v.verify(&sign(&claims(), None), "raw-nonce").await),
            "no kid"
        );
        // HS256 (alg confusion) is refused outright.
        let hs = encode(
            &Header {
                kid: Some(KID.into()),
                ..Header::new(Algorithm::HS256)
            },
            &claims(),
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();
        assert!(invalid(v.verify(&hs, "raw-nonce").await));
        assert!(v.cache.read().await.is_none(), "no JWKS fetch happened");
    }

    #[tokio::test]
    async fn unknown_kid_within_refetch_window_is_invalid_without_network() {
        let v = google_with_test_key().await;
        let res = v
            .verify(&sign(&claims(), Some("other-kid")), "raw-nonce")
            .await;
        assert_eq!(res, Err(VerifyError::Invalid("unknown key id")));
    }

    #[tokio::test]
    async fn signature_from_another_key_is_rejected() {
        let v = google_with_test_key().await;
        let token = sign(&claims(), Some(KID));
        // Flip a character in the signature segment.
        let (head, sig) = token.rsplit_once('.').unwrap();
        let mut sig: Vec<u8> = sig.bytes().collect();
        sig[5] = if sig[5] == b'A' { b'B' } else { b'A' };
        let forged = format!("{head}.{}", String::from_utf8(sig).unwrap());
        assert!(matches!(
            v.verify(&forged, "raw-nonce").await,
            Err(VerifyError::Invalid(_))
        ));
    }

    #[test]
    fn apple_verifier_uses_apple_issuer_and_service_id() {
        let v = OidcVerifier::apple("com.example.ouril.web".into(), None);
        assert_eq!(v.provider, "apple");
        assert_eq!(v.issuers, vec!["https://appleid.apple.com".to_string()]);
        assert_eq!(v.audiences, vec!["com.example.ouril.web".to_string()]);
        // Native iOS tokens use the bundle ID.
        let v = OidcVerifier::apple(
            "com.example.ouril.web".into(),
            Some("com.example.ouril".into()),
        );
        assert_eq!(
            v.audiences,
            vec![
                "com.example.ouril.web".to_string(),
                "com.example.ouril".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn cache_answers_without_refetching() {
        let v = google_with_test_key().await;
        let cache = v.cache.read().await;
        let (_, set) = cache.as_ref().unwrap();
        // Fresh set: a known kid is a key, an unknown one is rejected without a fetch.
        let fresh = (Instant::now(), set.clone());
        assert!(matches!(cached_key(Some(&fresh), KID), Some(Ok(_))));
        assert!(matches!(
            cached_key(Some(&fresh), "other"),
            Some(Err(VerifyError::Invalid("unknown key id")))
        ));
        // Empty cache: fetch.
        assert!(cached_key(None, KID).is_none());
        // Older than the refetch interval: an unknown kid may fetch again; a known one is used.
        if let Some(t) = Instant::now().checked_sub(JWKS_MIN_REFETCH + Duration::from_secs(1)) {
            let older = (t, set.clone());
            assert!(cached_key(Some(&older), "other").is_none());
            assert!(matches!(cached_key(Some(&older), KID), Some(Ok(_))));
        }
        // Past the TTL: always fetch.
        if let Some(t) = Instant::now().checked_sub(JWKS_TTL + Duration::from_secs(1)) {
            assert!(cached_key(Some(&(t, set.clone())), KID).is_none());
        }
    }
}
