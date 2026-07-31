//! Launch arguments that change what a shortcut actually starts.
//!
//! Separate from `identity` because this is command-line parsing, not identity derivation:
//! `identity` asks "which application is this", this file answers "were these two started in
//! different modes" — the distinction that keeps a browser profile shortcut from merging with
//! the browser itself.

use super::target::normalize_path;

pub(super) fn meaningful_launch_arguments(value: Option<&str>) -> Option<String> {
    let tokens = tokenize_quoted_arguments(value?);
    let mut meaningful = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index].trim_matches('"').to_lowercase();
        let takes_value = matches!(
            token.as_str(),
            "--profile-directory"
                | "--user-data-dir"
                | "--app"
                | "--app-id"
                | "--class"
                | "-p"
                | "/k"
                | "/c"
                | "-c"
                | "-command"
                | "-file"
        );
        let inline = [
            "--profile-directory=",
            "--user-data-dir=",
            "--app=",
            "--app-id=",
            "--class=",
        ]
        .iter()
        .any(|prefix| token.starts_with(prefix));
        let standalone = matches!(
            token.as_str(),
            "--safe-mode" | "--incognito" | "--private-window" | "--guest" | "--kiosk"
        );
        if standalone {
            meaningful.push(token);
        } else if inline {
            let (key, value) = token.split_once('=').expect("inline argument has equals");
            meaningful.push(format!("{key}={}", normalize_argument_value(key, value)));
        } else if takes_value {
            meaningful.push(token);
            if let Some(next) = tokens.get(index + 1) {
                meaningful.push(normalize_argument_value(
                    meaningful.last().expect("argument key was added"),
                    next,
                ));
                index += 1;
            }
        }
        index += 1;
    }
    (!meaningful.is_empty()).then(|| meaningful.join(" "))
}

fn normalize_argument_value(key: &str, value: &str) -> String {
    let value = value.trim_matches('"');
    if key == "--user-data-dir" {
        normalize_path(value)
    } else {
        value.to_lowercase()
    }
}

fn tokenize_quoted_arguments(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quoted = false;
    for character in value.chars() {
        match character {
            '"' => quoted = !quoted,
            character if character.is_whitespace() && !quoted => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            character => token.push(character),
        }
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}
