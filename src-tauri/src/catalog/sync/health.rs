use crate::catalog::source::{
    SourceErrorKind, SourceHealth, SourceHealthState, SourceKey, SourceSnapshot,
};
use crate::catalog::sync::scan_control;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) struct SourceOutcome {
    pub key: &'static str,
    pub stop: Option<scan_control::StageStop>,
    pub answered: bool,
    pub replaced: bool,
    pub records: usize,
    pub duration: Duration,
}

pub(super) fn seconds_since_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map_or(0, |duration| duration.as_secs())
}

pub(super) fn source_health(
    previous: &[SourceSnapshot],
    outcome: SourceOutcome,
    attempted_at: u64,
) -> SourceHealth {
    let key = SourceKey(outcome.key.to_string());
    let completed = outcome.answered && outcome.stop.is_none();
    let cancelled = outcome.stop == Some(scan_control::StageStop::Cancelled);
    let last_success_at = crate::catalog::source::previous_success(previous, &key);
    let failures = crate::catalog::source::previous_failures(previous, &key);
    let served = previous
        .iter()
        .find(|snapshot| snapshot.key == key)
        .map_or(0, |snapshot| snapshot.apps.len());
    let replaced = outcome.replaced && outcome.stop.is_none();

    let state = if completed {
        SourceHealthState::Fresh
    } else if last_success_at.is_none() {
        SourceHealthState::FailedWithoutSnapshot
    } else if outcome.stop.is_some() {
        SourceHealthState::Incomplete
    } else {
        SourceHealthState::Stale
    };

    SourceHealth {
        state,
        last_attempt_at: Some(attempted_at),
        last_success_at: if completed {
            Some(attempted_at)
        } else {
            last_success_at
        },
        consecutive_failures: match (completed, cancelled) {
            (true, _) => 0,
            (false, true) => failures,
            (false, false) => failures.saturating_add(1),
        },
        last_duration_ms: u64::try_from(outcome.duration.as_millis()).ok(),
        last_error: if completed {
            None
        } else {
            outcome
                .stop
                .map(SourceErrorKind::from)
                .or(Some(SourceErrorKind::ProviderFailed))
        },
        record_count: if replaced { outcome.records } else { served },
        key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::sync::scan_control::StageStop;

    const NOW: u64 = 1_700_000_000;

    fn outcome(stop: Option<StageStop>, answered: bool, records: usize) -> SourceOutcome {
        SourceOutcome {
            key: "start-apps",
            stop,
            answered,
            replaced: answered && stop.is_none(),
            records,
            duration: Duration::from_millis(12),
        }
    }

    fn served(apps: usize, health: Option<SourceHealth>) -> Vec<SourceSnapshot> {
        vec![SourceSnapshot {
            key: SourceKey("start-apps".into()),
            fingerprint: None,
            apps: (0..apps)
                .map(|index| super::super::app(&format!("app-{index}"), "Served"))
                .collect(),
            health,
        }]
    }

    fn healthy(last_success_at: u64, failures: u32) -> SourceHealth {
        SourceHealth {
            key: SourceKey("start-apps".into()),
            state: SourceHealthState::Fresh,
            last_attempt_at: Some(last_success_at),
            last_success_at: Some(last_success_at),
            consecutive_failures: failures,
            last_duration_ms: Some(5),
            last_error: None,
            record_count: 3,
        }
    }

    #[test]
    fn a_completed_attempt_is_fresh_and_clears_the_failure_streak() {
        let previous = served(3, Some(healthy(NOW - 500, 4)));

        let health = source_health(&previous, outcome(None, true, 7), NOW);

        assert_eq!(health.state, SourceHealthState::Fresh);
        assert_eq!(health.last_success_at, Some(NOW));
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.last_error, None);
        assert_eq!(health.record_count, 7);
    }

    #[test]
    fn an_empty_successful_result_is_not_a_failure() {
        let previous = served(3, Some(healthy(NOW - 500, 0)));

        let health = source_health(&previous, outcome(None, true, 0), NOW);

        assert_eq!(health.state, SourceHealthState::Fresh);
        assert_eq!(health.record_count, 0);
        assert_eq!(health.last_error, None);
    }

    #[test]
    fn a_provider_that_did_not_answer_keeps_serving_and_reports_stale() {
        let previous = served(3, Some(healthy(NOW - 500, 1)));

        let health = source_health(&previous, outcome(None, false, 0), NOW);

        assert_eq!(health.state, SourceHealthState::Stale);
        assert_eq!(health.last_success_at, Some(NOW - 500));
        assert_eq!(health.consecutive_failures, 2);
        assert_eq!(health.last_error, Some(SourceErrorKind::ProviderFailed));
        assert_eq!(health.record_count, 3, "it still serves what it had");
    }

    #[test]
    fn a_stage_that_stopped_early_reports_incomplete_with_its_reason() {
        let previous = served(3, Some(healthy(NOW - 500, 0)));

        let health = source_health(&previous, outcome(Some(StageStop::TimedOut), true, 1), NOW);

        assert_eq!(health.state, SourceHealthState::Incomplete);
        assert_eq!(health.last_error, Some(SourceErrorKind::TimedOut));
        assert_eq!(health.record_count, 3);
    }

    #[test]
    fn a_cancelled_steam_scan_keeps_the_previous_record_count() {
        let previous = vec![SourceSnapshot {
            key: SourceKey("steam".into()),
            fingerprint: None,
            apps: (0..3)
                .map(|index| super::super::app(&format!("app-{index}"), "Served"))
                .collect(),
            health: Some(healthy(NOW - 500, 0)),
        }];
        let outcome = SourceOutcome {
            key: "steam",
            stop: Some(StageStop::Cancelled),
            answered: true,
            replaced: true,
            records: 1,
            duration: Duration::from_millis(12),
        };

        let health = source_health(&previous, outcome, NOW);

        assert_eq!(health.state, SourceHealthState::Incomplete);
        assert_eq!(health.record_count, 3);
    }

    #[test]
    fn cancellation_does_not_count_as_a_failure() {
        let previous = served(3, Some(healthy(NOW - 500, 2)));

        let health = source_health(&previous, outcome(Some(StageStop::Cancelled), true, 0), NOW);

        assert_eq!(health.consecutive_failures, 2);
        assert_eq!(health.last_error, Some(SourceErrorKind::Cancelled));
    }

    #[test]
    fn a_source_that_never_succeeded_is_distinguishable_from_a_stale_one() {
        let health = source_health(&[], outcome(None, false, 0), NOW);

        assert_eq!(health.state, SourceHealthState::FailedWithoutSnapshot);
        assert_eq!(health.last_success_at, None);
        assert_eq!(health.consecutive_failures, 1);
    }
}
