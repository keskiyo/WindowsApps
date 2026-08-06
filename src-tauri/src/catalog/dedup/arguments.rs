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

/// The executable a Squirrel updater stub is asked to start: `Update.exe --processStart App.exe`.
///
/// Deliberately *not* part of `meaningful_launch_arguments`: this argument does not put the product
/// into a different mode, it names the product. Treating it as a mode would make the stub and the
/// application disagree on `launch_mode`, which `evidence::should_merge` vetoes outright — the
/// opposite of what recognizing it is for.
pub(super) fn squirrel_process_start(value: Option<&str>) -> Option<String> {
    let tokens = tokenize_quoted_arguments(value?);
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index].trim_matches('"').to_lowercase();
        let started = match token.split_once('=') {
            Some((key, value)) if is_process_start(key) => Some(value.to_owned()),
            Some(_) => None,
            None if is_process_start(&token) => tokens
                .get(index + 1)
                .map(|next| next.trim_matches('"').to_lowercase()),
            None => None,
        };
        // A bare file name is what Squirrel writes; anything carrying a path is not this layout,
        // so it is rejected rather than guessed at.
        if let Some(started) = started.filter(|value| {
            value.ends_with(".exe") && !value.contains('\\') && !value.contains('/')
        }) {
            return Some(started);
        }
        index += 1;
    }
    None
}

fn is_process_start(token: &str) -> bool {
    matches!(
        token,
        "--processstart" | "--processstartandwait" | "-processstart"
    )
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
