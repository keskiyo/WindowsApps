use std::cell::Cell;
use std::time::{Duration, Instant};

pub(crate) const DEFAULT_STAGE_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) const START_MENU_MAX_DEPTH: usize = 8;

pub(crate) const START_MENU_MAX_ENTRIES: usize = 50_000;

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
        if Instant::now() >= self.deadline {
            self.stop.set(Some(StageStop::TimedOut));
            return false;
        }
        true
    }

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
