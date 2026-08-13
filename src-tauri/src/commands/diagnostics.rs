const MAX_KIND_LENGTH: usize = 120;
const MAX_DETAIL_LENGTH: usize = 2000;

fn sanitize(value: &str, limit: usize) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || *character == '\n')
        .take(limit)
        .collect::<String>()
        .trim()
        .to_string()
}

#[tauri::command]
pub(crate) fn log_client_error(kind: String, detail: String) {
    let kind = sanitize(&kind, MAX_KIND_LENGTH);
    let detail = sanitize(&detail, MAX_DETAIL_LENGTH);
    if kind.is_empty() && detail.is_empty() {
        return;
    }
    log::error!("Interface recovery: {kind} {detail}");
}

#[cfg(test)]
mod tests {
    use super::{sanitize, MAX_DETAIL_LENGTH, MAX_KIND_LENGTH};

    #[test]
    fn a_report_is_bounded_and_free_of_control_characters() {
        let noisy = format!("Type\u{0}Error\r\n{}", "x".repeat(MAX_DETAIL_LENGTH * 2));

        let cleaned = sanitize(&noisy, MAX_DETAIL_LENGTH);

        assert_eq!(cleaned.chars().count(), MAX_DETAIL_LENGTH);
        assert!(!cleaned.contains('\u{0}'));
        assert!(!cleaned.contains('\r'));
        assert!(cleaned.starts_with("TypeError\n"));
    }

    #[test]
    fn a_kind_keeps_its_own_shorter_bound() {
        assert_eq!(
            sanitize(&"k".repeat(MAX_KIND_LENGTH + 50), MAX_KIND_LENGTH).len(),
            MAX_KIND_LENGTH
        );
    }

    #[test]
    fn whitespace_only_input_collapses_to_nothing() {
        assert!(sanitize("  \n  ", MAX_DETAIL_LENGTH).is_empty());
    }
}
