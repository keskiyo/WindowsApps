//! The current user's Windows UI language reduced to the one distinction the catalog acts on: is
//! it Russian or not. A merged card can carry both a localized and an English name (a Russian
//! Start-Menu shortcut plus an English registry entry); this decides which the user should see.
//!
//! The product ships two card languages, Russian and English, and nothing else. Everyone who is
//! not on a Russian Windows reads English, which is also the language of the entire interface.

use windows::Win32::Globalization::GetUserDefaultLocaleName;

/// Writing system of a display name or the OS UI language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameScript {
    Latin,
    Cyrillic,
    Other,
}

/// Script of the current user's Windows UI language: `Cyrillic` for Russian, `Latin` for every
/// other language. Falls back to `Latin` when the locale cannot be read — the safe default, since
/// English is the language the whole audience shares.
pub(crate) fn os_ui_script() -> NameScript {
    // LOCALE_NAME_MAX_LENGTH is 85 wide chars, including the null terminator.
    let mut buffer = [0u16; 85];
    // SAFETY: `buffer` is a fixed 85-element array and we pass the whole slice, so the API knows
    // its exact capacity and cannot write past it. It fills a null-terminated UTF-16 locale name
    // and returns the count written (including the null) or 0 on failure; no handle or ownership
    // is transferred.
    let written = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    if written <= 0 {
        return NameScript::Latin;
    }
    let end = (written as usize).saturating_sub(1).min(buffer.len());
    let name = String::from_utf16_lossy(&buffer[..end]).to_lowercase();
    locale_name_script(&name)
}

/// Pure classification of a BCP-47 locale name (`ru-ru`, `sr-cyrl-rs`, `en-us`, `kk-kz`).
///
/// Russian is the only language that reads a localized card name; every other language reads the
/// English one. That is the product's language policy, not a limitation of this function: the
/// interface itself is English throughout because it is the one language the whole audience shares,
/// so a Ukrainian or Serbian-Cyrillic user seeing Cyrillic card names inside an English interface
/// would be reading a mix, not a translation.
fn locale_name_script(name: &str) -> NameScript {
    let primary = name.split('-').next().unwrap_or(name);
    if primary == "ru" {
        NameScript::Cyrillic
    } else {
        NameScript::Latin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_russian_reads_localized_names() {
        for name in ["ru", "ru-ru", "ru-by", "ru-kz"] {
            assert_eq!(locale_name_script(name), NameScript::Cyrillic, "{name}");
        }
        // Every other language, Cyrillic-script or not, reads the English name.
        for name in [
            "en-us",
            "de-de",
            "ja-jp",
            "uk-ua",
            "kk-kz",
            "sr-cyrl-rs",
            "mn-mn",
            "bg-bg",
        ] {
            assert_eq!(locale_name_script(name), NameScript::Latin, "{name}");
        }
    }
}
