//! Handles generated for new accounts: the first word of the display name, folded to `[a-z0-9]`,
//! then six random digits (`Alfredo Moreira` → `alfredo934728`). Players can change theirs
//! later (`PATCH /v1/me`); a generated handle always passes the same rules
//! ([`crate::validate::handle`]).

use uuid::Uuid;

/// Random digits after the name: six, so even a common first name has 900,000 handles.
const DIGITS: usize = 6;
/// The longest name part, so a handle stays within the 20 characters handles allow.
const MAX_BASE: usize = 20 - DIGITS;
/// The name part when a display name has nothing to keep (no Latin letters or digits).
const FALLBACK: &str = "player";

/// A new handle for `display_name`: its name part and six random digits.
pub fn generate(display_name: &str) -> String {
    // A v4 UUID's random bits (122 of them); the modulo bias over 900,000 values is negligible.
    let digits = (Uuid::new_v4().as_u128() % 900_000) as u32 + 100_000;
    format!("{}{digits}", base_from_name(display_name))
}

/// The name part: the display name's first word, lowercased, with Latin accents folded
/// (`Cesária` → `cesaria`, `João` → `joao`) and everything else outside `[a-z0-9]` dropped.
/// At most 14 characters; `player` when nothing is left.
pub fn base_from_name(display_name: &str) -> String {
    let first = display_name.split_whitespace().next().unwrap_or("");
    let mut out = String::new();
    for c in first.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if let Some(folded) = fold(c) {
            out.push_str(folded);
        }
        if out.len() >= MAX_BASE {
            out.truncate(MAX_BASE);
            break;
        }
    }
    if out.is_empty() {
        FALLBACK.to_string()
    } else {
        out
    }
}

/// A Latin letter with a diacritic (or a ligature), as plain ASCII letters.
fn fold(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ď' | 'đ' | 'ð' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'ĵ' => "j",
        'ķ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'œ' => "oe",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'ß' => "ss",
        'ţ' | 'ť' | 'ŧ' => "t",
        'þ' => "th",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate;

    #[test]
    fn the_name_part_is_the_first_word_folded_to_ascii() {
        assert_eq!(base_from_name("Alfredo Moreira"), "alfredo");
        assert_eq!(base_from_name("  Cesária Évora "), "cesaria");
        assert_eq!(base_from_name("João"), "joao");
        assert_eq!(base_from_name("Maria-José"), "mariajose");
        assert_eq!(base_from_name("O'Neill"), "oneill");
        assert_eq!(base_from_name("ÆSA"), "aesa");
        assert_eq!(base_from_name("R2D2"), "r2d2");
    }

    #[test]
    fn a_name_with_nothing_to_keep_falls_back_to_player() {
        assert_eq!(base_from_name(""), "player");
        assert_eq!(base_from_name("   "), "player");
        assert_eq!(base_from_name("李小龙"), "player");
        assert_eq!(base_from_name("🙂"), "player");
    }

    #[test]
    fn a_long_name_is_cut_so_the_handle_fits() {
        assert_eq!(base_from_name("Bartholomewxyzqwerty"), "bartholomewxyz");
        assert_eq!(base_from_name("Bartholomewxyzqwerty").len(), MAX_BASE);
    }

    #[test]
    fn generated_handles_have_six_digits_and_pass_the_handle_rules() {
        for name in [
            "Alfredo Moreira",
            "Bartholomewxyzqwerty",
            "李小龙",
            "Al",
            "É",
        ] {
            let handle = generate(name);
            let digits = &handle[handle.len() - DIGITS..];
            assert!(digits.bytes().all(|b| b.is_ascii_digit()), "{handle}");
            assert!(!digits.starts_with('0'), "{handle}");
            assert_eq!(validate::handle(&handle).as_deref(), Ok(handle.as_str()));
        }
        assert!(generate("Alfredo").starts_with("alfredo"));
    }
}
