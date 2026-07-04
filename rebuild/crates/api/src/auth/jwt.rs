//! RS256 sign/verify + `kid` selection — ports `packages/platform/src/auth/jwt.ts` verbatim
//! (the crypto core the 2026-07-02 sweep rated "airtight — red tools bounce off"), with the two
//! FIXes the council froze folded in nowhere (this file carries the seam VERBATIM).
//!
//! ## The RS256 double-pin (proposal §1.1, jwt.ts:104-111) — ported as TWO independent guards
//! `jose`/`jsonwebtoken` in Node pin `algorithms: ['RS256']` in `jwtVerify` AND then throw again
//! if `protectedHeader.alg !== 'RS256'`. The sweep's instruction is explicit: "Port the
//! double-check verbatim; do not collapse it to a single guard." So `verify` here:
//!   1. builds a `Validation` whose `algorithms` is EXACTLY `[Algorithm::RS256]` (guard 1 — the
//!      library rejects any token whose header alg isn't in the set before checking the sig), and
//!   2. after a successful decode, independently asserts `header.alg == Algorithm::RS256` and
//!      errors otherwise (guard 2 — belt: catches an alg the library was somehow configured to
//!      allow).
//!
//! `verify_with_validation` takes the `Validation` as a parameter so a test can pass a
//! DELIBERATELY-BROADENED validation (RS256 + HS256) and prove guard 2 STILL rejects an HS256
//! token — i.e. the second guard is not dead code shadowed by the first (REV-2 double-pin vector).
//!
//! ## kid-selected key BEFORE verify (jwt.ts:87-102) + body-kid round-trip (REV-2/C1)
//! `decode_header` picks which trusted key to verify against by the (unverified) header `kid`;
//! the signature still gates acceptance. Unknown kid → "Invalid Key ID". The dev kid is accepted
//! ONLY in a `dev-routes` build AND `NODE_ENV != production` AND a dev keypair present — prod
//! holds no dev pubkey (three layers, ADR-0003).
//!
//! `kid` is ALSO a REQUIRED BODY claim (`legacy.ts:162`): `mint` writes it into the body (via
//! `Claims::finalize`) AND into the header. `verify` therefore checks that the decoded body's
//! `kid` matches the header `kid` — a header-only token (idiomatic `jsonwebtoken` mint that omits
//! the body claim) is rejected, and the strict `Claims` parse rejects it too since `kid` is a
//! required field. This is the seam that keeps Rust-minted and Node-minted tokens cross-verifiable
//! (resolution.md REV-2, breaker C1).

use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
    errors::ErrorKind,
};

use super::claims::Claims;
use super::config::AuthConfig;

/// Access-token / refresh-family TTLs, carried VERBATIM from the mint sites (proposal §5 TTL
/// matrix). Expressed in seconds so `mint` can stamp `exp = iat + ttl` deterministically.
pub mod ttl {
    pub const OWNER_ACCESS: i64 = 24 * 60 * 60; // 24h (ADR-0004 P-a)
    /// 7d dev bypass (local.ts:69) — bound only in a `dev-routes` build (the local-login dev path).
    #[allow(
        dead_code,
        reason = "dev-routes-only TTL — bound by the local-login dev bypass"
    )]
    pub const OWNER_DEV_ACCESS: i64 = 7 * 24 * 60 * 60;
    pub const COURIER_REDEEM_ACCESS: i64 = 14 * 24 * 60 * 60; // 14d (AUTH-GAP-3, courier/auth.ts:136)
    pub const COURIER_LOGIN_ACCESS: i64 = 24 * 60 * 60; // 24h (courier/auth.ts:335,465)
    pub const CUSTOMER_ACCESS: i64 = 7 * 24 * 60 * 60; // 7d (issueCustomerToken)
    /// 1d dev-kid (mock-auth.ts) — bound only in a `dev-routes` build (dev mock-auth).
    #[allow(dead_code, reason = "dev-routes-only TTL — bound by dev mock-auth")]
    pub const DEV_MOCK_ACCESS: i64 = 24 * 60 * 60;
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JwtError {
    /// `jwt.ts:98,101` — unknown/unaccepted `kid`.
    #[error("Invalid Key ID")]
    InvalidKeyId,
    /// `jwt.ts:109` — the belt alg guard tripped (should be unreachable via guard 1, proven live by test).
    #[error("Invalid algorithm — only RS256 accepted")]
    InvalidAlgorithm,
    /// Body-`kid` missing or != header `kid` (REV-2/C1) — a header-only token.
    #[error("Token body kid does not match header kid")]
    BodyKidMismatch,
    /// Signature invalid, expired, or the strict `Claims` parse failed — all collapse to one
    /// opaque "invalid" (never leak WHICH check failed; `plugins/auth.ts:54` logs, returns 401).
    #[error("Token expired or invalid")]
    Invalid,
    /// A mint-site misconfiguration (missing/invalid signing key). Never reachable from a request.
    #[error("signing key error: {0}")]
    Signing(String),
    /// ADR-0003: `signDevToken` throws if the dev keypair is absent — a dev mint site can never
    /// silently fall back to prod-key signing (`jwt.ts:76-78`).
    #[error("dev signing requires JWT_DEV_KID + dev keypair (non-prod only)")]
    DevSigningUnavailable,
}

/// The RS256 verifier + signer, built once from `AuthConfig` at boot. Holds the decoding keys so
/// per-request verify never re-parses PEM.
pub struct JwtVerifier {
    prod_kid: String,
    prod_decoding: DecodingKey,
    prod_encoding: EncodingKey,
    /// `Some` only in a `dev-routes` build with a dev keypair and `NODE_ENV != production`.
    #[cfg(feature = "dev-routes")]
    dev: Option<DevKeys>,
}

#[cfg(feature = "dev-routes")]
struct DevKeys {
    kid: String,
    decoding: DecodingKey,
    encoding: EncodingKey,
}

impl JwtVerifier {
    /// Builds the verifier from parsed config. Fails only on a malformed PEM (a boot-time
    /// misconfiguration), never on request data.
    pub fn from_config(cfg: &AuthConfig) -> Result<Self, JwtError> {
        let prod_decoding = DecodingKey::from_rsa_pem(cfg.jwt_public_key_pem.as_bytes())
            .map_err(|e| JwtError::Signing(format!("JWT_PUBLIC_KEY: {e}")))?;
        let prod_encoding = EncodingKey::from_rsa_pem(cfg.jwt_private_key_pem.as_bytes())
            .map_err(|e| JwtError::Signing(format!("JWT_PRIVATE_KEY: {e}")))?;

        #[cfg(feature = "dev-routes")]
        let dev = match (&cfg.dev, cfg.node_env.is_production()) {
            (Some(d), false) => {
                let decoding = DecodingKey::from_rsa_pem(d.jwt_dev_public_key_pem.as_bytes())
                    .map_err(|e| JwtError::Signing(format!("JWT_DEV_PUBLIC_KEY: {e}")))?;
                let encoding = EncodingKey::from_rsa_pem(d.jwt_dev_private_key_pem.as_bytes())
                    .map_err(|e| JwtError::Signing(format!("JWT_DEV_PRIVATE_KEY: {e}")))?;
                Some(DevKeys {
                    kid: d.jwt_dev_kid.clone(),
                    decoding,
                    encoding,
                })
            }
            _ => None,
        };

        Ok(JwtVerifier {
            prod_kid: cfg.jwt_kid.clone(),
            prod_decoding,
            prod_encoding,
            #[cfg(feature = "dev-routes")]
            dev,
        })
    }

    /// Signs a prod token — always the prod kid + prod private key (`signAuthToken`, jwt.ts:63).
    /// Stamps `iat = now`, `exp = now + ttl_secs`, `kid = prod_kid` into the body (and header) so
    /// the two can never diverge (REV-2/C1). Returns the compact JWT string.
    pub fn mint(&self, mut claims: Claims, ttl_secs: i64) -> Result<String, JwtError> {
        let now = chrono::Utc::now().timestamp();
        claims.finalize(now, now + ttl_secs, self.prod_kid.clone());
        self.sign(&claims, &self.prod_kid, &self.prod_encoding)
    }

    /// Signs a DEV token under the dev kid + dev keypair (`signDevToken`, jwt.ts:73-80). Only
    /// exists in a `dev-routes` build; errors `DevSigningUnavailable` if the dev keypair is
    /// absent — never falls back to the prod key (the ADR-0003 anti-backdoor property).
    #[cfg(feature = "dev-routes")]
    pub fn mint_dev(&self, mut claims: Claims, ttl_secs: i64) -> Result<String, JwtError> {
        let dev = self.dev.as_ref().ok_or(JwtError::DevSigningUnavailable)?;
        let now = chrono::Utc::now().timestamp();
        claims.finalize(now, now + ttl_secs, dev.kid.clone());
        self.sign(&claims, &dev.kid, &dev.encoding)
    }

    fn sign(&self, claims: &Claims, kid: &str, key: &EncodingKey) -> Result<String, JwtError> {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_string());
        encode(&header, claims, key).map_err(|e| JwtError::Signing(e.to_string()))
    }

    /// Verifies with the production-correct `Validation` (algorithms pinned to `[RS256]`, guard 1).
    /// The public per-request entry point.
    pub fn verify(&self, token: &str) -> Result<Claims, JwtError> {
        self.verify_with_validation(token, Self::rs256_validation())
    }

    /// The default production validation: alg pinned to RS256 ONLY (guard 1). Exposed for the
    /// double-pin test, which pairs it against a deliberately-broadened one.
    pub fn rs256_validation() -> Validation {
        let mut v = Validation::new(Algorithm::RS256);
        // Match Node's `jwtVerify(token, key, { algorithms: ['RS256'] })` — no audience/issuer
        // pinning (the Node verifier pins neither); `exp` is validated (jose + jsonwebtoken both
        // validate exp by default, matching the Node behavior where an expired token throws).
        v.required_spec_claims.clear();
        v.validate_exp = true;
        v.validate_aud = false;
        // jose-parity: Node's `jwtVerify` uses a clock tolerance of 0 by default, so a token whose
        // `exp` is even 1s in the past is rejected. `jsonwebtoken` defaults `leeway` to 60s — pin it
        // to 0 so the two stacks agree on expiry to the second (Phase-B cross-verify seam).
        v.leeway = 0;
        v.algorithms = vec![Algorithm::RS256];
        v
    }

    /// The core verify, parameterized on `Validation` so guard 2 can be proven independent of
    /// guard 1 (REV-2 double-pin vector). Steps mirror `verifyAuthToken` (jwt.ts:82-114):
    ///   1. `decode_header` → pick the trusted key by header `kid` (BEFORE the sig check).
    ///   2. `decode` with the given `Validation` (guard 1 lives here when the caller passes the
    ///      RS256-only validation).
    ///   3. independent `header.alg == RS256` assertion (guard 2 — belt).
    ///   4. body-`kid` == header-`kid` assertion (REV-2/C1).
    ///   5. return the strict-parsed `Claims` (deny-unknown / required-claim already enforced by
    ///      the `Claims` deserializer that `decode` ran).
    pub fn verify_with_validation(
        &self,
        token: &str,
        validation: Validation,
    ) -> Result<Claims, JwtError> {
        let header = decode_header(token).map_err(|_e| JwtError::Invalid)?;
        let header_kid = header.kid.clone().ok_or(JwtError::InvalidKeyId)?;

        let decoding_key = self.select_key(&header_kid)?;

        let data = decode::<Claims>(token, decoding_key, &validation).map_err(map_decode_err)?;

        // Guard 2 (belt): independent of guard 1's algorithm set. Even if `validation.algorithms`
        // was (mis)configured to include a non-RS256 alg, this rejects it.
        if data.header.alg != Algorithm::RS256 {
            return Err(JwtError::InvalidAlgorithm);
        }

        // REV-2/C1: body-`kid` is a required claim and MUST equal the header `kid`. A token that
        // carries `kid` only in the header (no body claim) is already rejected by the strict
        // `Claims` parse above (kid is required); this additionally rejects a token whose body
        // `kid` disagrees with the header used to select the key.
        if data.claims.kid() != header_kid {
            return Err(JwtError::BodyKidMismatch);
        }

        Ok(data.claims)
    }

    /// kid → trusted decoding key (jwt.ts:93-102). Prod kid → prod pubkey. Dev kid → dev pubkey
    /// ONLY in a dev-routes build with the dev keypair present (already gated to non-prod at
    /// `from_config`). Anything else → InvalidKeyId.
    fn select_key(&self, header_kid: &str) -> Result<&DecodingKey, JwtError> {
        if header_kid == self.prod_kid {
            return Ok(&self.prod_decoding);
        }
        #[cfg(feature = "dev-routes")]
        if let Some(dev) = &self.dev {
            if header_kid == dev.kid {
                return Ok(&dev.decoding);
            }
        }
        Err(JwtError::InvalidKeyId)
    }
}

/// Map `jsonwebtoken` decode errors onto the opaque `Invalid` (never leak which check failed) —
/// EXCEPT an immutable-key error, which is a mint-side/config bug worth its own variant.
fn map_decode_err(err: jsonwebtoken::errors::Error) -> JwtError {
    match err.kind() {
        // The strict Claims parse failed (unknown/missing field), sig invalid, or expired — all
        // one opaque "invalid" to the caller (T-3: strictness is a live control; a rejected token
        // is a rejected token, the reason is not leaked).
        ErrorKind::InvalidToken
        | ErrorKind::InvalidSignature
        | ErrorKind::ExpiredSignature
        | ErrorKind::Json(_)
        | ErrorKind::Base64(_)
        | ErrorKind::Utf8(_)
        | ErrorKind::InvalidAlgorithm
        | ErrorKind::InvalidAlgorithmName => JwtError::Invalid,
        _ => JwtError::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::claims::{CourierClaims, CustomerClaims, OwnerClaims};
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use uuid::Uuid;

    // Throwaway RSA-2048 keypairs, GENERATED once at test runtime (never committed key material —
    // repo secrets guardrail). `test_priv`/`test_pub` = the "prod" keypair; the dev-routes tests
    // additionally use a SECOND, distinct "dev" keypair.
    #[cfg(feature = "dev-routes")]
    use crate::test_support::keys::{dev_private as test_dev_priv, dev_public as test_dev_pub};
    use crate::test_support::keys::{prod_private as test_priv, prod_public as test_pub};
    // An HS256 secret for the alg-confusion vector.
    const HS_SECRET: &[u8] = b"attacker-symmetric-secret";

    fn verifier() -> JwtVerifier {
        JwtVerifier {
            prod_kid: "prod-kid-1".to_string(),
            prod_decoding: DecodingKey::from_rsa_pem(test_pub().as_bytes()).unwrap(),
            prod_encoding: EncodingKey::from_rsa_pem(test_priv().as_bytes()).unwrap(),
            #[cfg(feature = "dev-routes")]
            dev: None,
        }
    }

    fn owner_claims() -> Claims {
        Claims::Owner(OwnerClaims::new(Uuid::new_v4(), Some(Uuid::new_v4())))
    }

    #[test]
    fn mint_then_verify_round_trips_owner() {
        let v = verifier();
        let token = v.mint(owner_claims(), ttl::OWNER_ACCESS).unwrap();
        let claims = v.verify(&token).unwrap();
        assert_eq!(claims.role(), crate::auth::claims::Role::Owner);
        assert_eq!(claims.kid(), "prod-kid-1");
    }

    #[test]
    fn mint_writes_kid_to_body_and_header_and_they_match() {
        // REV-2/C1: a Rust-minted token carries kid in BOTH the body and the header, and a normal
        // verify (which cross-checks them) accepts it.
        let v = verifier();
        let token = v.mint(owner_claims(), ttl::OWNER_ACCESS).unwrap();
        let header = decode_header(&token).unwrap();
        assert_eq!(header.kid.as_deref(), Some("prod-kid-1"), "kid in header");
        let claims = v.verify(&token).unwrap();
        assert_eq!(claims.kid(), "prod-kid-1", "kid in body");
    }

    #[test]
    fn header_only_kid_token_is_rejected() {
        // REV-2/C1: an idiomatic mint that puts kid ONLY in the header (no body claim) fails —
        // here the body has NO kid field at all, so the strict Claims parse rejects it (kid is
        // required). Proven by hand-encoding a claims object without kid.
        let v = verifier();
        let body = serde_json::json!({
            "role": "owner", "userId": Uuid::new_v4(), "sub": Uuid::new_v4(),
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600,
            // NO kid in the body
        });
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("prod-kid-1".to_string());
        let token = encode(
            &header,
            &body,
            &EncodingKey::from_rsa_pem(test_priv().as_bytes()).unwrap(),
        )
        .unwrap();
        assert_eq!(v.verify(&token), Err(JwtError::Invalid));
    }

    #[test]
    fn body_kid_mismatch_is_rejected() {
        // REV-2/C1: body kid present but != header kid → BodyKidMismatch (defense beyond the
        // strict parse). Header selects the prod key; body claims a different kid string.
        let v = verifier();
        let body = serde_json::json!({
            "role": "owner", "userId": Uuid::new_v4(), "sub": Uuid::new_v4(),
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600,
            "kid": "some-other-kid",
        });
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("prod-kid-1".to_string());
        let token = encode(
            &header,
            &body,
            &EncodingKey::from_rsa_pem(test_priv().as_bytes()).unwrap(),
        )
        .unwrap();
        assert_eq!(v.verify(&token), Err(JwtError::BodyKidMismatch));
    }

    #[test]
    fn rs256_double_pin_second_guard_is_independent_of_the_first() {
        // REV-2 double-pin vector: forge an HS256 token, then verify it with a DELIBERATELY
        // BROADENED validation (RS256 + HS256) so guard 1 (the library's algorithm set) does NOT
        // reject it on algorithm grounds. Guard 2 (the independent post-decode header.alg == RS256
        // assertion) MUST still reject it. If guard 2 were removed/collapsed, this token would be
        // accepted — so this test fails iff either guard is deleted.
        let v = verifier();
        let body = serde_json::json!({
            "role": "owner", "userId": Uuid::new_v4(), "sub": Uuid::new_v4(),
            "iat": chrono::Utc::now().timestamp(),
            "exp": chrono::Utc::now().timestamp() + 3600,
            "kid": "prod-kid-1",
        });
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("prod-kid-1".to_string());
        let hs_token = encode(&header, &body, &EncodingKey::from_secret(HS_SECRET)).unwrap();

        // MISCONFIGURE guard 1 to ACCEPT the HS256 token: a validation that allows HS256 + an HMAC
        // key that matches that alg. (`jsonwebtoken` cross-checks the key family against every alg
        // in `validation.algorithms`, so a `[RS256, HS256]` list with an HMAC key would itself
        // error `InvalidAlgorithm` inside the library — i.e. the library can't be tricked into
        // decoding a family-mismatched token, which is a good property but NOT what we're isolating
        // here. To exercise guard 2 alone, we make guard 1 genuinely PASS the token.)
        let mut misconfigured = JwtVerifier::rs256_validation();
        misconfigured.algorithms = vec![Algorithm::HS256];

        let hs_verifier = JwtVerifier {
            prod_kid: "prod-kid-1".to_string(),
            prod_decoding: DecodingKey::from_secret(HS_SECRET),
            prod_encoding: EncodingKey::from_rsa_pem(test_priv().as_bytes()).unwrap(),
            #[cfg(feature = "dev-routes")]
            dev: None,
        };
        // With guard 1 misconfigured to accept HS256, `decode` SUCCEEDS (valid HS256 signature).
        // Guard 2 — the independent post-decode `header.alg == RS256` assertion — MUST still reject
        // it. If guard 2 were removed/collapsed into guard 1, this token would be accepted. So this
        // assertion fails iff the SECOND, independent guard is gone (REV-2 double-pin vector).
        assert_eq!(
            hs_verifier.verify_with_validation(&hs_token, misconfigured),
            Err(JwtError::InvalidAlgorithm),
            "guard 2 (independent header.alg==RS256) must reject an HS256 token that guard 1 was misconfigured to accept"
        );

        // And the ordinary RS256-only validation ALSO rejects it (guard 1), against the real
        // prod verifier — the two guards are both live.
        assert_eq!(v.verify(&hs_token), Err(JwtError::Invalid));
    }

    #[test]
    fn unknown_kid_is_rejected() {
        let v = verifier();
        let mut claims = owner_claims();
        claims.finalize(
            chrono::Utc::now().timestamp(),
            chrono::Utc::now().timestamp() + 3600,
            "unknown-kid".to_string(),
        );
        // Sign with the prod key but a header kid the verifier doesn't know.
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("unknown-kid".to_string());
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(test_priv().as_bytes()).unwrap(),
        )
        .unwrap();
        assert_eq!(v.verify(&token), Err(JwtError::InvalidKeyId));
    }

    #[test]
    fn expired_token_is_rejected() {
        let v = verifier();
        let mut claims = owner_claims();
        let past = chrono::Utc::now().timestamp() - 10;
        claims.finalize(past - 3600, past, "prod-kid-1".to_string());
        let token = v.sign(&claims, "prod-kid-1", &v.prod_encoding).unwrap();
        assert_eq!(v.verify(&token), Err(JwtError::Invalid));
    }

    #[test]
    fn tampered_signature_is_rejected() {
        let v = verifier();
        let token = v.mint(owner_claims(), ttl::OWNER_ACCESS).unwrap();
        // Flip the last char of the signature segment.
        let mut chars: Vec<char> = token.chars().collect();
        let last = chars.len() - 1;
        chars[last] = if chars[last] == 'A' { 'B' } else { 'A' };
        let tampered: String = chars.into_iter().collect();
        assert_eq!(v.verify(&tampered), Err(JwtError::Invalid));
    }

    #[test]
    fn customer_token_never_carries_phone_end_to_end() {
        // T-8: mint a customer token and prove the decoded body has no phone claim.
        let v = verifier();
        let claims = Claims::Customer(CustomerClaims::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ));
        let token = v.mint(claims, ttl::CUSTOMER_ACCESS).unwrap();
        let verified = v.verify(&token).unwrap();
        let json = serde_json::to_value(&verified).unwrap();
        assert!(json.get("phone").is_none());
    }

    #[test]
    fn courier_token_round_trips_with_jti() {
        let v = verifier();
        let jti = Uuid::new_v4();
        let claims = Claims::Courier(CourierClaims::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Some(jti),
        ));
        let token = v.mint(claims, ttl::COURIER_LOGIN_ACCESS).unwrap();
        let verified = v.verify(&token).unwrap();
        assert_eq!(verified.as_courier().unwrap().jti, Some(jti));
    }

    #[cfg(feature = "dev-routes")]
    #[test]
    fn dev_signed_token_is_rejected_by_a_verifier_with_no_dev_key() {
        // ADR-0003 / T-7: a dev-kid token must NOT verify on a binary that holds no dev pubkey
        // (the "prod verifier rejects dev tokens" property). Build a dev-signed token, verify it
        // on a verifier whose `dev` is None (the prod shape).
        let dev_encoding = EncodingKey::from_rsa_pem(test_dev_priv().as_bytes()).unwrap();
        let mut claims = owner_claims();
        claims.finalize(
            chrono::Utc::now().timestamp(),
            chrono::Utc::now().timestamp() + 3600,
            "dev-kid-1".to_string(),
        );
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("dev-kid-1".to_string());
        let dev_token = encode(&header, &claims, &dev_encoding).unwrap();

        let prod_only = verifier(); // dev: None
        assert_eq!(
            prod_only.verify(&dev_token),
            Err(JwtError::InvalidKeyId),
            "a verifier with no dev key must reject a dev-kid token by kid selection"
        );
    }

    #[cfg(feature = "dev-routes")]
    #[test]
    fn dev_verifier_accepts_dev_kid_and_rejects_prod_signed_dev_kid() {
        // In a dev build with the dev keypair, a dev-kid token minted with the DEV key verifies;
        // a token claiming the dev kid but signed with the PROD key does NOT (kid+key are a pair —
        // C.1 invariant, jwt.ts:48-60).
        let dev_decoding = DecodingKey::from_rsa_pem(test_dev_pub().as_bytes()).unwrap();
        let dev_encoding = EncodingKey::from_rsa_pem(test_dev_priv().as_bytes()).unwrap();
        let v = JwtVerifier {
            prod_kid: "prod-kid-1".to_string(),
            prod_decoding: DecodingKey::from_rsa_pem(test_pub().as_bytes()).unwrap(),
            prod_encoding: EncodingKey::from_rsa_pem(test_priv().as_bytes()).unwrap(),
            dev: Some(DevKeys {
                kid: "dev-kid-1".to_string(),
                decoding: dev_decoding,
                encoding: dev_encoding.clone(),
            }),
        };
        let token = v.mint_dev(owner_claims(), ttl::DEV_MOCK_ACCESS).unwrap();
        assert!(
            v.verify(&token).is_ok(),
            "dev-key-signed dev-kid token verifies in a dev build"
        );

        // dev kid claimed, but signed with the PROD key → signature fails against the dev pubkey.
        let mut claims = owner_claims();
        claims.finalize(
            chrono::Utc::now().timestamp(),
            chrono::Utc::now().timestamp() + 3600,
            "dev-kid-1".to_string(),
        );
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("dev-kid-1".to_string());
        let forged = encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(test_priv().as_bytes()).unwrap(),
        )
        .unwrap();
        assert_eq!(v.verify(&forged), Err(JwtError::Invalid));
    }
}
