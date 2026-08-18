use std::collections::{HashSet, VecDeque};

#[derive(Default)]
pub(crate) struct HydrationQueue {
    generation: u64,
    foreground: VecDeque<String>,
    background: VecDeque<String>,
    queued: HashSet<String>,
    running: bool,
}

impl HydrationQueue {
    pub(crate) fn enqueue(
        &mut self,
        generation: u64,
        ids: impl IntoIterator<Item = String>,
        priority: bool,
    ) -> bool {
        if self.generation != generation {
            self.generation = generation;
            self.foreground.clear();
            self.background.clear();
            self.queued.clear();
            self.running = false;
        }
        for id in ids {
            if !self.queued.insert(id.clone()) {
                if priority {
                    let was_background = self.background.iter().any(|queued| queued == &id);
                    self.background.retain(|queued| queued != &id);
                    if was_background && !self.foreground.iter().any(|queued| queued == &id) {
                        self.foreground.push_back(id);
                    }
                }
                continue;
            }
            if priority {
                self.foreground.push_back(id);
            } else {
                self.background.push_back(id);
            }
        }
        if self.running || self.queued.is_empty() {
            false
        } else {
            self.running = true;
            true
        }
    }

    pub(crate) fn pop(&mut self, generation: u64) -> Option<String> {
        if self.generation != generation {
            return None;
        }
        self.foreground
            .pop_front()
            .or_else(|| self.background.pop_front())
    }

    pub(crate) fn complete(&mut self, generation: u64, id: &str) {
        if self.generation == generation {
            self.queued.remove(id);
        }
    }

    pub(crate) fn finish(&mut self, generation: u64) {
        if self.generation == generation {
            self.running = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_prioritizes_visible_ids_and_deduplicates_requests() {
        let mut queue = HydrationQueue::default();
        assert!(queue.enqueue(1, ["a".into(), "b".into(), "c".into()], false));
        assert!(!queue.enqueue(1, ["c".into(), "b".into()], true));
        assert_eq!(queue.pop(1).as_deref(), Some("c"));
        queue.complete(1, "c");
        assert_eq!(queue.pop(1).as_deref(), Some("b"));
        queue.complete(1, "b");
        assert_eq!(queue.pop(1).as_deref(), Some("a"));
    }

    #[test]
    fn new_generation_discards_stale_hydration_work() {
        let mut queue = HydrationQueue::default();
        assert!(queue.enqueue(1, ["old".into()], false));
        assert!(queue.enqueue(2, ["new".into()], true));
        assert_eq!(queue.pop(1), None);
        assert_eq!(queue.pop(2).as_deref(), Some("new"));
    }

    #[test]
    fn visible_request_does_not_duplicate_an_in_flight_id() {
        let mut queue = HydrationQueue::default();
        assert!(queue.enqueue(1, ["app".into()], false));
        assert_eq!(queue.pop(1).as_deref(), Some("app"));
        assert!(!queue.enqueue(1, ["app".into()], true));
        queue.complete(1, "app");
        assert_eq!(queue.pop(1), None);
    }
}
