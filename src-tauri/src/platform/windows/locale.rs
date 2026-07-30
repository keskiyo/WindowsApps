//! The current user's Windows UI language reduced to the one distinction the catalog acts on: is
//! it a Cyrillic-script locale or not. A merged card can carry both a localized and an English name
//! (a Russian Start-Menu shortcut plus an English registry entry); this decides which script the
//! user should see so a non-Russian user is never shown Cyrillic when a Latin name exists.

use windows::Win32::Globalization::GetUserDefaultLocaleName;

/// Writing system of a display name or the OS UI language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameScript {
    Latin,
    Cyrillic,
    Other,
}

/// Script of the current user's Windows UI language: `Cyrillic` for Russian and the other
/// Cyrillic-script locales, otherwise `Latin`. Falls back to `Latin` when the locale cannot be
/// read — the safe default that never forces a non-Russian user to read Cyrillic.
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
fn locale_name_script(name: &str) -> NameScript {
    if name.contains("cyrl") {
        return NameScript::Cyrillic;
    }
    // Latin-script variants are tagged explicitly (`sr-latn-…`, `uz-latn-…`); trust that over the
    // language subtag, which is otherwise ambiguous for a few languages.
    if name.contains("latn") {
        return NameScript::Latin;
    }
    const CYRILLIC_LANGUAGES: &[&str] = &[
        "ru", "uk", "be", "bg", "mk", "sr", "kk", "ky", "tg", "mn", "tt", "ba", "cv", "sah", "ce",
        "os", "ab", "kv",
    ];
    let primary = name.split('-').next().unwrap_or(name);
    if CYRILLIC_LANGUAGES.contains(&primary) {
        NameScript::Cyrillic
    } else {
        NameScript::Latin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_locale_names_to_script() {
        for name in ["ru-ru", "uk-ua", "kk-kz", "sr-cyrl-rs", "mn-mn"] {
            assert_eq!(locale_name_script(name), NameScript::Cyrillic, "{name}");
        }
        for name in ["en-us", "de-de", "sr-latn-rs", "uz-latn-uz", "ja-jp"] {
            assert_eq!(locale_name_script(name), NameScript::Latin, "{name}");
        }
    }
}
