//! Field validation for profile and settings input. Rules marked "provisional" are open
//! questions in docs/product/features/accounts.md and API.md.

/// Display name: trimmed, 1..=50 characters, no control characters, no invisible format
/// characters (bidi overrides, zero-width characters, BOM: they allow spoofing once names are
/// shown to other players) and no line/paragraph separators.
pub fn display_name(v: &str) -> Result<String, &'static str> {
    let v = v.trim();
    let n = v.chars().count();
    if n == 0 || n > 50 {
        return Err("display_name must be 1 to 50 characters");
    }
    if v.chars().any(char::is_control) {
        return Err("display_name must not contain control characters");
    }
    if v.chars()
        .any(|c| is_format_char(c) || c == '\u{2028}' || c == '\u{2029}')
    {
        return Err("display_name must not contain invisible formatting characters");
    }
    Ok(v.to_string())
}

/// Unicode general category Cf (format characters), as of Unicode 16.
fn is_format_char(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

/// Handle (provisional): `[a-z0-9_]{3,20}`, case-insensitive (stored lowercase), leading `@`
/// tolerated.
pub fn handle(v: &str) -> Result<String, &'static str> {
    let v = v.trim().trim_start_matches('@').to_ascii_lowercase();
    let len = v.len();
    if !(3..=20).contains(&len)
        || !v
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err("handle must match [a-z0-9_]{3,20}");
    }
    Ok(v)
}

/// Avatar URL: `https://` only, at most 2048 bytes.
pub fn avatar_url(v: &str) -> Result<String, &'static str> {
    let v = v.trim();
    if v.len() > 2048 || !v.starts_with("https://") || v.len() <= "https://".len() {
        return Err("avatar_url must be an https:// URL of at most 2048 bytes");
    }
    if v.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("avatar_url must not contain whitespace");
    }
    Ok(v.to_string())
}

/// Locale: a BCP 47-shaped tag (`en`, `pt`, `kea`, `pt-CV`), 2..=35 characters.
pub fn locale(v: &str) -> Result<String, &'static str> {
    let v = v.trim();
    let ok = (2..=35).contains(&v.len())
        && v.split('-').all(|part| {
            !part.is_empty() && part.len() <= 8 && part.bytes().all(|b| b.is_ascii_alphanumeric())
        })
        && v.split('-').next().is_some_and(|p| {
            (2..=3).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphabetic())
        });
    if !ok {
        return Err("locale must be a BCP 47 language tag such as en, pt or kea");
    }
    Ok(v.to_string())
}

/// Country: ISO 3166-1 alpha-2, stored lowercase.
pub fn country(v: &str) -> Result<String, &'static str> {
    let v = v.trim().to_ascii_lowercase();
    if v.len() != 2 || !v.bytes().all(|b| b.is_ascii_lowercase()) {
        return Err("country must be an ISO 3166-1 alpha-2 code");
    }
    Ok(v)
}

/// Dev sign-in user key: `[a-z0-9_-]{1,32}`.
pub fn dev_user(v: &str) -> Result<String, &'static str> {
    if v.is_empty()
        || v.len() > 32
        || !v
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
    {
        return Err("user must match [a-z0-9_-]{1,32}");
    }
    Ok(v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles() {
        assert_eq!(handle("@Mindelo_Master").unwrap(), "mindelo_master");
        assert!(handle("ab").is_err());
        assert!(handle("has space").is_err());
        assert!(handle("ção").is_err());
        assert!(handle(&"a".repeat(21)).is_err());
    }

    #[test]
    fn names_locales_countries() {
        assert_eq!(display_name("  Ana  ").unwrap(), "Ana");
        assert!(display_name("").is_err());
        assert!(display_name("a\u{0007}b").is_err());
        // Accented and non-Latin names are fine.
        assert_eq!(display_name("João Évora").unwrap(), "João Évora");
        assert!(display_name("Ana 🎲").is_ok());
        // Bidi overrides/isolates, zero-width characters, BOM, soft hyphen, separators.
        for bad in [
            "Ana\u{202E}gnp.exe",
            "Ana\u{2067}x\u{2069}",
            "A\u{200B}na",
            "A\u{200D}na",
            "\u{FEFF}Ana",
            "A\u{00AD}na",
            "Ana\u{2028}Lopes",
            "Ana\u{E0041}",
        ] {
            assert!(display_name(bad).is_err(), "{bad:?}");
        }
        assert!(locale("kea").is_ok());
        assert!(locale("pt-CV").is_ok());
        assert!(locale("e").is_err());
        assert!(locale("en_US").is_err());
        assert_eq!(country("CV").unwrap(), "cv");
        assert!(country("cpv").is_err());
        assert!(avatar_url("http://x.example/a.png").is_err());
        assert!(avatar_url("https://x.example/a.png").is_ok());
        assert!(dev_user("dev").is_ok());
        assert!(dev_user("Dev").is_err());
    }
}
