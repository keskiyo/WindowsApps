use super::corpus::generated_records;
use crate::catalog::machine::Registrations;
use crate::catalog::storage::cache::{self, CatalogCache};
use crate::platform::windows::NameScript;
use std::time::{Duration, Instant};

const REPEATS: usize = 5;

const CATALOG_SIZE: usize = 2000;

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn measure(label: &str, mut run: impl FnMut()) {
    run();
    let samples = (0..REPEATS)
        .map(|_| {
            let started = Instant::now();
            run();
            started.elapsed()
        })
        .collect::<Vec<_>>();
    println!(
        "{label}: median {median:?} of {samples:?}",
        median = median(samples.clone())
    );
}

#[test]
#[ignore = "developer-only measurement: prints medians instead of asserting a threshold"]
fn stage_timings() {
    let records = generated_records(0x5EED, CATALOG_SIZE);
    println!("catalog: {} records", records.len());

    measure("clone only (subtract from the stages below)", || {
        let _ = std::hint::black_box(records.clone());
    });
    measure("sanitize", || {
        let _ = std::hint::black_box(crate::catalog::sanitize_pinned(
            records.clone(),
            &Registrations::empty(),
            NameScript::Latin,
        ));
    });
    measure("dedup", || {
        let _ = std::hint::black_box(crate::catalog::dedup::resolved_groups(
            super::visible_entries(records.clone()),
        ));
    });

    let directory = tempfile::tempdir().expect("a temporary directory");
    let document = CatalogCache {
        apps: crate::catalog::sanitize_pinned(
            records.clone(),
            &Registrations::empty(),
            NameScript::Latin,
        ),
        ..CatalogCache::default()
    };
    measure("cache write", || {
        let _ = cache::write_document(directory.path(), &document);
    });
    measure("cache load", || {
        let _ = std::hint::black_box(cache::read_document(directory.path()));
    });
}
