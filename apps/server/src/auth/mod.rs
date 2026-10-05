//! Authentication: our own sessions (access JWT + rotating refresh token), provider ID-token
//! verification (Google/Apple placeholders) and the dev identity provider.
//! See docs/product/features/accounts.md and ADR 0009.

pub mod apple;
pub mod cookie;
pub mod extractor;
pub mod oidc;
pub mod session;
pub mod tokens;

pub use extractor::AuthUser;
