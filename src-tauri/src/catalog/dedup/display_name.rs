use crate::platform::windows::NameScript;

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
