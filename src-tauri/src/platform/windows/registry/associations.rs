use std::collections::BTreeMap;
use std::path::Path;
use winreg::enums::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

const REGISTERED_APPLICATIONS: &[(winreg::HKEY, &str)] = &[
    (HKEY_LOCAL_MACHINE, r"SOFTWARE\RegisteredApplications"),
    (HKEY_CURRENT_USER, r"SOFTWARE\RegisteredApplications"),
];

const MAX_APPLICATIONS: usize = 512;
const MAX_TOKENS_PER_EXECUTABLE: usize = 64;

pub(crate) fn registered_associations() -> Vec<(String, Vec<String>)> {
    let mut collected: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (stem, token) in supported_types().chain(capability_associations()) {
        let tokens = collected.entry(stem).or_default();
        if tokens.len() < MAX_TOKENS_PER_EXECUTABLE && !tokens.contains(&token) {
            tokens.push(token);
        }
    }
    collected.into_iter().collect()
}

fn supported_types() -> impl Iterator<Item = (String, String)> {
    let root = RegKey::predef(HKEY_CLASSES_ROOT).open_subkey("Applications");
    let names = root
        .as_ref()
        .map(|key| {
            key.enum_keys()
                .filter_map(Result::ok)
                .take(MAX_APPLICATIONS)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    names.into_iter().flat_map(move |name| {
        let stem = executable_stem(&name);
        let types = root
            .as_ref()
            .ok()
            .and_then(|key| key.open_subkey(format!(r"{name}\SupportedTypes")).ok())
            .map(|key| {
                key.enum_values()
                    .filter_map(Result::ok)
                    .map(|(value, _)| value)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        types
            .into_iter()
            .filter_map(move |extension| {
                let token = file_token(&extension)?;
                Some((stem.clone(), token))
            })
            .collect::<Vec<_>>()
    })
}

fn capability_associations() -> impl Iterator<Item = (String, String)> {
    let mut pairs = Vec::new();
    for (hive, subkey) in REGISTERED_APPLICATIONS {
        let Ok(root) = RegKey::predef(*hive).open_subkey(subkey) else {
            continue;
        };
        for (_, capabilities) in root
            .enum_values()
            .filter_map(Result::ok)
            .take(MAX_APPLICATIONS)
        {
            let path = capabilities.to_string();
            let Ok(key) = RegKey::predef(*hive).open_subkey(path.trim()) else {
                continue;
            };
            collect_capability(&key, "FileAssociations", file_token, &mut pairs);
            collect_capability(&key, "UrlAssociations", protocol_token, &mut pairs);
        }
    }
    pairs.into_iter()
}

fn collect_capability(
    capabilities: &RegKey,
    name: &str,
    token_of: fn(&str) -> Option<String>,
    pairs: &mut Vec<(String, String)>,
) {
    let Ok(key) = capabilities.open_subkey(name) else {
        return;
    };
    for (association, program_id) in key.enum_values().filter_map(Result::ok) {
        let Some(token) = token_of(&association) else {
            continue;
        };
        let Some(stem) = program_executable(&program_id.to_string()) else {
            continue;
        };
        pairs.push((stem, token));
    }
}

fn program_executable(program_id: &str) -> Option<String> {
    let command: String = RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(format!(r"{}\shell\open\command", program_id.trim()))
        .ok()?
        .get_value("")
        .ok()?;
    let trimmed = command.trim();
    let quoted = trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.split('"').next());
    let executable =
        quoted.unwrap_or_else(|| trimmed.split_whitespace().next().unwrap_or_default());
    let stem = executable_stem(executable);
    (!stem.is_empty()).then_some(stem)
}

fn executable_stem(value: &str) -> String {
    Path::new(value.trim().trim_matches('"'))
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

fn file_token(extension: &str) -> Option<String> {
    let normalized = extension.trim().to_lowercase();
    let normalized = normalized.strip_prefix('.')?;
    (!normalized.is_empty() && normalized.chars().all(|c| c.is_ascii_alphanumeric()))
        .then(|| format!(".{normalized}"))
}

fn protocol_token(protocol: &str) -> Option<String> {
    let normalized = protocol.trim().trim_end_matches(':').to_lowercase();
    (!normalized.is_empty() && normalized.chars().all(|c| c.is_ascii_alphanumeric()))
        .then(|| format!("url:{normalized}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_normalized_and_reject_junk() {
        assert_eq!(file_token(".MP3"), Some(".mp3".into()));
        assert_eq!(file_token("  .Psd "), Some(".psd".into()));
        assert_eq!(file_token("mp3"), None);
        assert_eq!(file_token(".tar.gz"), None);
        assert_eq!(protocol_token("mailto:"), Some("url:mailto".into()));
        assert_eq!(protocol_token("TG"), Some("url:tg".into()));
        assert_eq!(protocol_token(""), None);
    }

    #[test]
    fn an_executable_stem_survives_quotes_and_paths() {
        assert_eq!(executable_stem(r#""C:\Program Files\VLC\vlc.exe""#), "vlc");
        assert_eq!(executable_stem("Notepad++.exe"), "notepad++");
        assert_eq!(executable_stem(""), "");
    }

    #[test]
    fn reading_the_live_registry_stays_bounded_and_normalized() {
        let associations = registered_associations();

        assert!(associations.len() <= MAX_APPLICATIONS * 2);
        for (stem, tokens) in &associations {
            assert!(!stem.is_empty());
            assert_eq!(stem.to_lowercase(), *stem);
            assert!(tokens.len() <= MAX_TOKENS_PER_EXECUTABLE);
            for token in tokens {
                assert!(
                    token.starts_with('.') || token.starts_with("url:"),
                    "{token}"
                );
            }
        }
    }
}
