use windows::Win32::Globalization::GetUserDefaultLocaleName;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameScript {
    Latin,
    Cyrillic,
    Other,
}

pub(crate) fn os_ui_script() -> NameScript {
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
