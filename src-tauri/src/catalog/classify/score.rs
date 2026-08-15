use super::super::AppCategory;
use super::signals::Signals;
use super::tables::RULES;

pub(super) const DECISIVE: i32 = 120;
pub(super) const PUBLISHER: i32 = 100;
pub(super) const FEATURE: i32 = 90;
pub(super) const INSTALL_TREE: i32 = 80;
pub(super) const EXE: i32 = 70;
pub(super) const EXE_GLUE: i32 = 60;
pub(super) const PRODUCT: i32 = 40;
pub(super) const DESC: i32 = 30;
pub(super) const NAME: i32 = 25;
pub(super) const PATH: i32 = 25;
pub(super) const VOCABULARY: i32 = 22;

pub(super) const THRESHOLD: i32 = 20;

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

const MAX_EVIDENCE: usize = 3;

pub(super) fn evidence(signals: &Signals, category: AppCategory) -> Vec<String> {
    let mut matched = RULES
        .iter()
        .filter(|rule| rule.category == category)
        .filter_map(|rule| {
            let needle = rule
                .needles
                .iter()
                .find(|needle| signals.matches(rule.field, needle))?;
            Some((rule.weight, format!("{}={needle}", rule.field.id())))
        })
        .collect::<Vec<_>>();
    matched.sort_by_key(|(weight, _)| std::cmp::Reverse(*weight));
    matched
        .into_iter()
        .take(MAX_EVIDENCE)
        .map(|(_, reason)| reason)
        .collect()
}

fn category_score(signals: &Signals, category: AppCategory) -> i32 {
    RULES
        .iter()
        .filter(|rule| rule.category == category)
        .filter(|rule| {
            rule.needles
                .iter()
                .any(|needle| signals.matches(rule.field, needle))
        })
        .map(|rule| rule.weight)
        .sum()
}

#[cfg(test)]
pub(super) fn ranked(signals: &Signals) -> Vec<(AppCategory, i32)> {
    let mut scores: Vec<(AppCategory, i32)> = PRIORITY
        .iter()
        .map(|&category| (category, category_score(signals, category)))
        .filter(|(_, score)| *score > 0)
        .collect();
    scores.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    scores
}

pub(super) fn best(signals: &Signals) -> AppCategory {
    let mut best_category = AppCategory::Other;
    let mut best_score = 0;
    for &category in PRIORITY {
        let score = category_score(signals, category);
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
