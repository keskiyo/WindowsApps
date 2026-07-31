//! Choosing which of a merged card's names to show.
//!
//! Deliberately independent of which record wins the merge: the launching source, icon and target
//! come from the highest-scored candidate, but a Russian shortcut on an English machine should
//! still read in Latin. Only the displayed name follows the OS language.

use crate::platform::windows::NameScript;

/// Writing system of a display name — only the distinction the locale rule acts on. Any Cyrillic
/// letter marks a localized name; otherwise a Latin letter marks an English/Latin name.
pub(super) fn name_script(name: &str) -> NameScript {
    let mut has_latin = false;
    for character in name.chars() {
        if ('\u{0400}'..='\u{052F}').contains(&character) {
            return NameScript::Cyrillic;
        }
        if character.is_ascii_alphabetic() {
            has_latin = true;
        }
    }
    if has_latin {
        NameScript::Latin
    } else {
        NameScript::Other
    }
}

/// Chooses the display name for a merged card from all of its candidate names, given the OS UI
/// script. `primary` is the highest-scored source's name (kept when it already matches). Otherwise
/// a candidate in the OS script wins; failing that, any Latin (English) name — so a non-Cyrillic
/// user never ends up reading a Cyrillic card when a Latin alternative exists. Purely a naming
/// choice: the launching source, icon, and target are unchanged.
pub(super) fn choose_display_name<'a>(
    primary: &str,
    candidates: impl Iterator<Item = &'a str>,
    os_script: NameScript,
) -> String {
    if name_script(primary) == os_script {
        return primary.to_string();
    }
    let names = candidates.collect::<Vec<_>>();
    if let Some(matched) = names.iter().find(|name| name_script(name) == os_script) {
        return (*matched).to_string();
    }
    if name_script(primary) == NameScript::Latin {
        return primary.to_string();
    }
    if let Some(latin) = names
        .iter()
        .find(|name| name_script(name) == NameScript::Latin)
    {
        return (*latin).to_string();
    }
    primary.to_string()
}
