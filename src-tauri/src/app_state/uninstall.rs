use crate::catalog::{SourceKind, UninstallTarget};
use crate::platform::windows::{uninstall_history, uninstaller};
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct UninstallRecord {
    pub(crate) app_name: String,
    pub(crate) publisher: Option<String>,
    pub(crate) source_kind: SourceKind,
    pub(crate) target: UninstallTarget,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UninstallPreview {
    pub(crate) app_name: String,
    pub(crate) publisher: Option<String>,
    pub(crate) source: SourceKind,
    pub(crate) mechanism: uninstaller::UninstallMechanism,
}

pub(crate) fn preview_for(record: &UninstallRecord) -> UninstallPreview {
    let target = uninstaller::preview(&record.target);
    UninstallPreview {
        app_name: record.app_name.clone(),
        publisher: record.publisher.clone(),
        source: record.source_kind,
        mechanism: target.mechanism,
    }
}

pub(crate) fn execute_and_record(
    app_data_dir: &Path,
    record: UninstallRecord,
    executor: impl FnOnce(UninstallTarget) -> Result<uninstaller::UninstallOutcome, String>,
) -> Result<uninstaller::UninstallOutcome, String> {
    let preview = uninstaller::preview(&record.target);
    let result = executor(record.target);
    if result == Ok(uninstaller::UninstallOutcome::Cancelled) {
        return result;
    }
    let history_result = if result.is_ok() {
        uninstall_history::UninstallResult::Succeeded
    } else {
        uninstall_history::UninstallResult::Failed
    };
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let _ = uninstall_history::append(
        app_data_dir,
        uninstall_history::UninstallHistoryEntry {
            id: String::new(),
            timestamp,
            app_name: record.app_name,
            publisher: record.publisher,
            mechanism: preview.mechanism,
            result: history_result,
        },
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uninstall_record(name: &str) -> UninstallRecord {
        UninstallRecord {
            app_name: name.into(),
            publisher: Some("Publisher".into()),
            source_kind: SourceKind::Registry,
            target: UninstallTarget::Command {
                executable: "uninstall.exe".into(),
                arguments: "/quiet".into(),
            },
        }
    }

    #[test]
    fn records_successful_uninstall_without_command_details() {
        let dir = tempfile::tempdir().unwrap();
        execute_and_record(dir.path(), uninstall_record("Editor"), |_| {
            Ok(uninstaller::UninstallOutcome::Completed)
        })
        .unwrap();

        let history = uninstall_history::read(dir.path());
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].app_name, "Editor");
        assert_eq!(
            history[0].result,
            uninstall_history::UninstallResult::Succeeded
        );
        let serialized = serde_json::to_value(&history[0]).unwrap();
        assert!(serialized.get("command").is_none());
        assert!(serialized.get("path").is_none());
        assert!(serialized.get("error").is_none());
    }

    #[test]
    fn uninstall_preview_excludes_command_details() {
        let serialized = serde_json::to_value(preview_for(&uninstall_record("Editor"))).unwrap();

        assert_eq!(serialized.get("command"), None);
        assert_eq!(serialized.get("path"), None);
        assert_eq!(serialized["mechanism"], "registered_command");
    }

    // A wizard the user closed did nothing, so the history must not claim an attempt failed.
    #[test]
    fn a_cancelled_uninstall_is_not_written_to_history() {
        let dir = tempfile::tempdir().unwrap();

        let outcome = execute_and_record(dir.path(), uninstall_record("Editor"), |_| {
            Ok(uninstaller::UninstallOutcome::Cancelled)
        })
        .unwrap();

        assert_eq!(outcome, uninstaller::UninstallOutcome::Cancelled);
        assert!(uninstall_history::read(dir.path()).is_empty());
    }

    #[test]
    fn records_failed_uninstall_and_returns_original_error() {
        let dir = tempfile::tempdir().unwrap();
        let error = execute_and_record(dir.path(), uninstall_record("Editor"), |_| {
            Err("boom".into())
        })
        .unwrap_err();

        let history = uninstall_history::read(dir.path());
        assert_eq!(error, "boom");
        assert_eq!(
            history[0].result,
            uninstall_history::UninstallResult::Failed
        );
    }
}
