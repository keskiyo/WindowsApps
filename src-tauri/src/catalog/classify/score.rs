//! Weighted scoring engine. Every rule that matches adds its weight to its category; the category
//! with the highest total wins, provided it clears `THRESHOLD` (otherwise the app is `Other`). Ties
//! resolve by a fixed priority so the outcome is deterministic. Corroborating signals accumulate —
//! several weak hits can outrank a single stronger one — which is the point of scoring over a cascade.

use super::super::AppCategory;
use super::signals::Signals;
use super::tables::RULES;

/// Signal strength, decisive → weak. A rule in the tables tags itself with one of these.
pub(super) const DECISIVE: i32 = 120; // a narrow override that must beat a publisher (PDF vs Adobe)
pub(super) const PUBLISHER: i32 = 100;
pub(super) const FEATURE: i32 = 90; // verbatim inbox/Windows-feature marker
pub(super) const INSTALL_TREE: i32 = 80;
pub(super) const EXE: i32 = 70; // resolved executable stem
pub(super) const EXE_GLUE: i32 = 60; // marker glued into an executable name (SotaVPN → vpn)
pub(super) const PRODUCT: i32 = 40; // PE product name
pub(super) const DESC: i32 = 30; // PE file description
pub(super) const NAME: i32 = 25; // display name
pub(super) const PATH: i32 = 25; // a dedicated store/game install folder

/// Minimum winning score. A lone weak signal (a generic path hit) stays below it and falls to Other.
const THRESHOLD: i32 = 20;

/// Tie-break order, mirroring the intent of the previous first-match cascade: the earlier category
/// wins when two totals are equal.
const PRIORITY: &[AppCategory] = &[
    AppCategory::WindowsFeatures,
    AppCategory::Games,
    AppCategory::Ai,
    AppCategory::Development,
    AppCategory::Editors,
    AppCategory::Productivity,
    AppCategory::Security,
    AppCategory::FileCloud,
    AppCategory::Browsers,
    AppCategory::Media,
    AppCategory::Communication,
    AppCategory::System,
    AppCategory::Utilities,
];

pub(super) fn best(signals: &Signals) -> AppCategory {
    let mut best_category = AppCategory::Other;
    let mut best_score = 0;
    for &category in PRIORITY {
        let score: i32 = RULES
            .iter()
            .filter(|rule| rule.category == category)
            .filter(|rule| {
                rule.needles
                    .iter()
                    .any(|needle| signals.matches(rule.field, needle))
            })
            .map(|rule| rule.weight)
            .sum();
        // Strictly greater keeps the earlier (higher-priority) category on ties.
        if score > best_score {
            best_score = score;
            best_category = category;
        }
    }
    if best_score >= THRESHOLD {
        best_category
    } else {
        AppCategory::Other
    }
}
