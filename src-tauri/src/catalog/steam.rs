use std::path::{Path, PathBuf};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SteamGame {
    pub app_id: String,
    pub name: String,
    pub install_dir: PathBuf,
}

pub(super) fn parse_library_paths(value: &str) -> Vec<PathBuf> {
    let tokens = quoted_values(value);
    let mut paths = tokens
        .windows(2)
        .filter(|pair| pair[0].eq_ignore_ascii_case("path"))
        .map(|pair| PathBuf::from(pair[1].replace(r"\\", r"\")))
        .collect::<Vec<_>>();
    paths.dedup();
    paths
}

pub(super) fn parse_manifest(value: &str, library: &Path) -> Option<SteamGame> {
    let tokens = quoted_values(value);
    let value_for = |key: &str| {
        tokens
            .windows(2)
            .find(|pair| pair[0].eq_ignore_ascii_case(key))
            .map(|pair| pair[1].clone())
    };
    let app_id = value_for("appid")?;
    let name = value_for("name")?;
    let install_dir = library
        .join("steamapps")
        .join("common")
        .join(value_for("installdir")?);
    Some(SteamGame {
        app_id,
        name,
        install_dir,
    })
}

pub(super) fn scan_library(library: &Path) -> Vec<SteamGame> {
    let Ok(entries) = std::fs::read_dir(library.join("steamapps")) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with("appmanifest_") && name.ends_with(".acf"))
        })
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .filter_map(|value| parse_manifest(&value, library))
        .collect()
}

/// Main Steam install directory (contains `appcache/librarycache`), from the registry.
pub(crate) fn steam_root() -> Option<PathBuf> {
    let steam = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .ok()?;
    let path = steam.get_value::<String, _>("SteamPath").ok()?;
    Some(PathBuf::from(path.replace('/', r"\")))
}

pub(super) fn installed_libraries() -> Vec<PathBuf> {
    let Some(main) = steam_root() else {
        return Vec::new();
    };
    let mut libraries = vec![main.clone()];
    if let Ok(value) = std::fs::read_to_string(main.join("steamapps").join("libraryfolders.vdf")) {
        libraries.extend(parse_library_paths(&value));
    }
    libraries.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
    libraries.dedup_by(|left, right| {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    });
    libraries
}

fn quoted_values(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut start = None;
    for (index, character) in value.char_indices() {
        if character != '"' {
            continue;
        }
        match start.take() {
            Some(begin) => values.push(value[begin..index].to_string()),
            None => start = Some(index + 1),
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_all_steam_library_paths_without_assuming_drive_letters() {
        let paths = parse_library_paths(
            r#""libraryfolders"
            {
                "0" { "path" "C:\\Program Files (x86)\\Steam" }
                "1" { "path" "E:\\My Games\\SteamLibrary" }
            }"#,
        );

        assert_eq!(
            paths,
            vec![
                PathBuf::from(r"C:\Program Files (x86)\Steam"),
                PathBuf::from(r"E:\My Games\SteamLibrary"),
            ]
        );
    }

    #[test]
    fn reads_game_identity_from_manifest() {
        let game = parse_manifest(
            r#""AppState" { "appid" "2183900" "name" "Warhammer 40,000: Space Marine 2" "installdir" "Space Marine 2" }"#,
            Path::new(r"D:\SteamLibrary"),
        ).unwrap();

        assert_eq!(game.app_id, "2183900");
        assert_eq!(game.name, "Warhammer 40,000: Space Marine 2");
        assert_eq!(
            game.install_dir,
            PathBuf::from(r"D:\SteamLibrary\steamapps\common\Space Marine 2")
        );
    }

    #[test]
    fn scans_every_manifest_in_a_library() {
        let library = tempfile::tempdir().unwrap();
        let steamapps = library.path().join("steamapps");
        std::fs::create_dir_all(&steamapps).unwrap();
        std::fs::write(
            steamapps.join("appmanifest_1942280.acf"),
            r#""AppState" { "appid" "1942280" "name" "Brotato" "installdir" "Brotato" }"#,
        )
        .unwrap();

        let games = scan_library(library.path());

        assert_eq!(games.len(), 1);
        assert_eq!(games[0].name, "Brotato");
    }
}
