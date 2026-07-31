//! Cooperative bounds shared by the Windows scan stages.
//!
//! Registry, Start Menu and Start-Apps each ran to completion before the first cancellation check,
//! with no time limit and no size limit. A wedged COM/AppX provider or a pathological Start Menu
//! tree therefore left Refresh looking cancelled while the worker still held the scan lock and the
//! external process kept running. `ScanControl` gives every stage the same three answers: has the
//! user cancelled, has this stage run out of time, and has it seen more entries than the source
//! can plausibly hold.

use std::cell::Cell;
use std::time::{Duration, Instant};

/// Ceiling for one Windows source. Large enough that a healthy machine never reaches it, small
/// enough that a wedged provider cannot hold the scan lock for minutes.
pub(crate) const DEFAULT_STAGE_TIMEOUT: Duration = Duration::from_secs(30);

/// A Start Menu is a shallow tree of program folders. Depth beyond this is a junction loop or a
/// corrupted profile, not a program layout.
pub(crate) const START_MENU_MAX_DEPTH: usize = 8;

/// Entry ceiling for one Start Menu traversal. A large developer machine reports a few thousand.
pub(crate) const START_MENU_MAX_ENTRIES: usize = 50_000;

/// Why a stage stopped early. `None` from [`StageBudget::stop`] means the stage ran to completion
/// and its snapshot may replace the stored one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StageStop {
    Cancelled,
    TimedOut,
    EntryLimit,
}

pub(crate) struct ScanControl<'a> {
    cancelled: &'a (dyn Fn() -> bool + 'a),
}

impl<'a> ScanControl<'a> {
    pub(crate) fn new(cancelled: &'a (dyn Fn() -> bool + 'a)) -> Self {
        Self { cancelled }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        (self.cancelled)()
    }

    pub(crate) fn stage(&self, timeout: Duration) -> StageBudget<'_> {
        self.stage_with(timeout, usize::MAX, usize::MAX)
    }

    pub(crate) fn stage_with(
        &self,
        timeout: Duration,
        max_entries: usize,
        max_depth: usize,
    ) -> StageBudget<'_> {
        StageBudget {
            cancelled: self.cancelled,
            deadline: Instant::now() + timeout,
            max_entries,
            max_depth,
            entries: Cell::new(0),
            stop: Cell::new(None),
        }
    }
}

/// One stage's remaining allowance. Uses interior mutability so it can be consulted from inside an
/// iterator adapter without threading a mutable borrow through the traversal.
pub(crate) struct StageBudget<'a> {
    cancelled: &'a (dyn Fn() -> bool + 'a),
    deadline: Instant,
    max_entries: usize,
    max_depth: usize,
    entries: Cell<usize>,
    stop: Cell<Option<StageStop>>,
}

impl StageBudget<'_> {
    pub(crate) fn max_depth(&self) -> usize {
        self.max_depth
    }

    /// Records and returns the first reason this stage must stop. Sticky: once a stage has stopped
    /// it stays stopped, so a traversal cannot resume after the deadline merely because a later
    /// clock reading was cheaper to skip.
    pub(crate) fn should_stop(&self) -> bool {
        if self.stop.get().is_some() {
            return true;
        }
        if (self.cancelled)() {
            self.stop.set(Some(StageStop::Cancelled));
            return true;
        }
        if Instant::now() >= self.deadline {
            self.stop.set(Some(StageStop::TimedOut));
            return true;
        }
        false
    }

    /// Accounts for one visited entry. Returns `false` once the stage must stop, so a traversal can
    /// use it directly as its loop condition.
    pub(crate) fn charge_entry(&self) -> bool {
        if self.stop.get().is_some() {
            return false;
        }
        let seen = self.entries.get() + 1;
        self.entries.set(seen);
        if seen > self.max_entries {
            self.stop.set(Some(StageStop::EntryLimit));
            return false;
        }
        if (self.cancelled)() {
            self.stop.set(Some(StageStop::Cancelled));
            return false;
        }
        // The clock is read on every entry rather than every Nth: a stage's per-entry work is a
        // shortcut resolution or a PE metadata read, so one QPC call is noise beside it, while
        // sampling would let a slow entry overrun the deadline by an unbounded margin.
        if Instant::now() >= self.deadline {
            self.stop.set(Some(StageStop::TimedOut));
            return false;
        }
        true
    }

    /// `None` means the stage ran to completion and may replace its stored snapshot. An incomplete
    /// one must not: its partial result would report every unvisited application as removed.
    pub(crate) fn stop(&self) -> Option<StageStop> {
        self.stop.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn never() -> bool {
        false
    }

    fn always() -> bool {
        true
    }

    #[test]
    fn a_fresh_stage_is_complete_and_permits_work() {
        let cancelled = never;
        let control = ScanControl::new(&cancelled);
        let stage = control.stage(DEFAULT_STAGE_TIMEOUT);

        assert!(!stage.should_stop());
        assert!(stage.charge_entry());
        assert_eq!(stage.stop(), None);
    }

    #[test]
    fn cancellation_stops_a_stage_before_any_entry_is_charged() {
        let cancelled = always;
        let control = ScanControl::new(&cancelled);
        let stage = control.stage(DEFAULT_STAGE_TIMEOUT);

        assert!(control.is_cancelled());
        assert!(stage.should_stop());
        assert!(!stage.charge_entry());
        assert_eq!(stage.stop(), Some(StageStop::Cancelled));
    }

    // Zero budget rather than a sleep: the deadline is already in the past when the stage starts,
    // so the test is deterministic and costs no wall-clock time.
    #[test]
    fn an_exhausted_deadline_stops_a_stage() {
        let cancelled = never;
        let control = ScanControl::new(&cancelled);
        let stage = control.stage(Duration::ZERO);

        assert!(stage.should_stop());
        assert_eq!(stage.stop(), Some(StageStop::TimedOut));
        assert!(!stage.charge_entry());
    }

    #[test]
    fn the_entry_cap_stops_a_stage_after_exactly_its_allowance() {
        let cancelled = never;
        let control = ScanControl::new(&cancelled);
        let stage = control.stage_with(DEFAULT_STAGE_TIMEOUT, 3, 4);

        assert!(stage.charge_entry());
        assert!(stage.charge_entry());
        assert!(stage.charge_entry());
        assert!(!stage.charge_entry());
        assert_eq!(stage.stop(), Some(StageStop::EntryLimit));
        assert_eq!(stage.max_depth(), 4);
    }

    // Whichever limit fires first owns the outcome; a later check must not relabel it.
    #[test]
    fn the_first_stop_reason_wins() {
        let cancelled = never;
        let control = ScanControl::new(&cancelled);
        let stage = control.stage_with(Duration::ZERO, 1, 1);

        assert!(stage.should_stop());
        assert_eq!(stage.stop(), Some(StageStop::TimedOut));
        assert!(!stage.charge_entry());
        assert_eq!(stage.stop(), Some(StageStop::TimedOut));
    }
}
