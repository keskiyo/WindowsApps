//! The text markers as data: every needle carries the field it reads, the tier of evidence that
//! field is, and the strongest outcome that tier is allowed to produce.
//!
//! Making the outcome part of the rule is the point. Before this, a needle's authority lived in
//! whichever `if` happened to test it, so "a word may not delete an application" was a convention a
//! reviewer had to keep in their head. Here the type says it: this table has no `Reject` outcome to
//! spell, so no word can remove a record — rejection is reachable only from the structural
//! predicates in `super::structural`. `local_corpus_rules_cannot_settle_a_class` enforces the
//! weaker half of the same idea for needles written against one machine.

use super::super::VisibilityReason;
use crate::catalog::fields::{Field, MarkerFields};

/// Where a needle's evidence comes from, which is what bounds its authority.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::catalog::visibility) enum Tier {
    /// Vendor-authored: a file name, a PE description, a product name. Reads the same on every
    /// machine, because the vendor wrote it once for all of them.
    VendorMetadata,
    /// A display name, or a directory shape. Localized, user-editable, but not specific to any one
    /// installation.
    DisplayVocabulary,
    /// Names one product, vendor, or path observed on the development machine. Correct where it
    /// was written; unproven anywhere else.
    LocalCorpus,
}

impl Tier {
    /// The most a needle written against one machine may weigh. Half the ordinary marker weight,
    /// so a local needle can tip a record that other evidence already left borderline and can
    /// never carry one across the threshold on its own.
    const LOCAL_CORPUS_CAP: i16 = 10;

    /// Invariant I2, applied by the engine rather than trusted to a reviewer: whatever weight a
    /// local-corpus rule declares, this is what it actually contributes.
    fn cap(self, weight: i16) -> i16 {
        match self {
            Self::LocalCorpus => weight.max(-Self::LOCAL_CORPUS_CAP),
            Self::VendorMetadata | Self::DisplayVocabulary => weight,
        }
    }
}

/// The strongest thing a rule may do. Deliberately missing a `Reject` variant.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::catalog::visibility) enum Outcome {
    /// Settles the record into Auxiliary tools, where it stays listed and one click restores it.
    ForceAuxiliary(i16),
    /// Adds weight and lets the class threshold decide.
    Weigh(i16),
}

pub(in crate::catalog::visibility) struct Rule {
    tier: Tier,
    pub(in crate::catalog::visibility) field: Field,
    pub(in crate::catalog::visibility) needles: &'static [&'static str],
    outcome: Outcome,
    pub(in crate::catalog::visibility) reason: VisibilityReason,
}

impl Rule {
    /// The rule's effect after its tier's cap is applied. Callers cannot reach the declared weight,
    /// so a rule can never contribute more than its evidence is allowed to.
    pub(in crate::catalog::visibility) fn effect(&self) -> Outcome {
        match self.outcome {
            Outcome::ForceAuxiliary(score) => Outcome::ForceAuxiliary(score),
            Outcome::Weigh(weight) => Outcome::Weigh(self.tier.cap(weight)),
        }
    }
}

/// A shortcut that opens documentation instead of a program. Name-scoped: as prose these words hid
/// real software — an install path containing `\manual\`, or a PE description mentioning a manual,
/// was enough to drop the application entirely.
const DOCUMENTATION_NAMES: &[&str] = &[
    " faq",
    "faqs",
    "documentation",
    "installation notes",
    "release notes",
    "what's new",
    "home page",
    "getting started",
    "visit website",
    "support center",
    "readme",
    "manual",
    "документация",
    "справка",
    "руководство",
];

/// Update, crash-reporting and housekeeping binaries that ship beside an application. The generic
/// words are bound to the file name on purpose: as free text each one names a real product —
/// Watchdog Anti-Malware, a driver updater, SQuirreL SQL Client.
const MAINTENANCE_FILE_NAMES: &[&str] = &[
    "update.exe",
    "update-service",
    "update_service",
    "autoupdate",
    "updater",
    "crashhandler",
    "crashpad",
    "uninstall.exe",
    "elevationservice",
    "werfault",
    "bugsplat",
    "telemetry",
    "watchdog",
    "squirrel.exe",
    "backgroundtaskhost",
    // The PTY/console host bundled by Terminal and Electron applications, not the user-facing
    // Windows Terminal launcher.
    "openconsole.exe",
];

const MAINTENANCE_NAMES: &[&str] = &[
    "crash handler",
    "elevation service",
    "bug report",
    "background task host",
    "деинстал",
    // "Reset preferences and cache files"-style shortcuts run the app in a wipe mode instead of
    // opening it; they belong with maintenance, not the player itself.
    "reset preferences",
    "reset cache",
    "reset config",
    "reset settings",
    "сброс настроек",
    "сброс кэш",
];

/// Components named by what they are, in the vendor's own file name.
const COMPONENT_FILE_NAMES: &[&str] = &[
    "crashpad_handler",
    "broker.exe",
    "pwa_launcher",
    "util.exe",
    "service.exe",
    // A binary named `notification_helper.exe` is a component whatever the shortcut pointing at it
    // was renamed to — the file name is the honest evidence here.
    "_helper",
    "-helper",
    "helper.exe",
];

/// Components named by the role they describe in their PE metadata.
const COMPONENT_PROSE: &[&str] = &[
    "language server",
    "openjdk platform binary",
    // Oracle Java runtime entries ("Java(TM) Platform SE") are the JRE, a runtime component.
    "java(tm)",
    "the curl executable",
    "openssl command",
    "credential manager",
    "gettext",
    "git-lfs",
    "git large file storage",
    " helper",
    "web helper",
    "subprocess",
    " daemon",
];

/// Specific binaries seen on the development machine. Kept for the coverage they carry, capped to
/// weight so neither can settle a class on a machine they were never written for.
const LOCAL_COMPONENT_FILE_NAMES: &[&str] = &["iconv.exe", "intelliphp.ls"];

/// Directories that hold a product's own runtime rather than the product.
const RUNTIME_DIRECTORIES: &[&str] = &[
    r"\bin\",
    r"\lib\",
    r"\runtime\",
    r"\jre\",
    r"\sdk\",
    r"\plugins\",
    r"\resources\",
    r"\node_modules\",
    r"\squirrel\",
    r"\packages\",
    r"\dotnet\",
];

/// Toolchains this machine happens to have installed.
const LOCAL_TOOLCHAIN_DIRECTORIES: &[&str] = &[
    r"\.vscode\extensions\",
    r"\.codeium\",
    r"\git\usr\bin\",
    r"\git\mingw32\",
    r"\git\mingw64\",
    r"\codex-runtimes\",
    r"\sdk\samples\",
];

const SDK_SAMPLE_DIRECTORIES: &[&str] = &[r"\sdk\samples\", r"\samples\sharedmemory\"];

const SDK_SAMPLE_PROSE: &[&str] = &["shared memory sample", "sample application"];

pub(in crate::catalog::visibility) const RULES: &[Rule] = &[
    Rule {
        tier: Tier::DisplayVocabulary,
        field: Field::Name,
        needles: DOCUMENTATION_NAMES,
        outcome: Outcome::ForceAuxiliary(-80),
        reason: VisibilityReason::DocumentationShortcut,
    },
    Rule {
        tier: Tier::VendorMetadata,
        field: Field::FileName,
        needles: MAINTENANCE_FILE_NAMES,
        outcome: Outcome::ForceAuxiliary(-70),
        reason: VisibilityReason::MaintenanceExecutable,
    },
    Rule {
        tier: Tier::DisplayVocabulary,
        field: Field::Name,
        needles: MAINTENANCE_NAMES,
        outcome: Outcome::ForceAuxiliary(-70),
        reason: VisibilityReason::MaintenanceExecutable,
    },
    Rule {
        tier: Tier::VendorMetadata,
        field: Field::FileName,
        needles: COMPONENT_FILE_NAMES,
        outcome: Outcome::Weigh(-20),
        reason: VisibilityReason::ProductComponent,
    },
    Rule {
        tier: Tier::VendorMetadata,
        field: Field::Prose,
        needles: COMPONENT_PROSE,
        outcome: Outcome::Weigh(-20),
        reason: VisibilityReason::ProductComponent,
    },
    Rule {
        tier: Tier::LocalCorpus,
        field: Field::FileName,
        needles: LOCAL_COMPONENT_FILE_NAMES,
        outcome: Outcome::Weigh(-20),
        reason: VisibilityReason::ProductComponent,
    },
    Rule {
        tier: Tier::LocalCorpus,
        field: Field::Path,
        needles: LOCAL_TOOLCHAIN_DIRECTORIES,
        outcome: Outcome::Weigh(-20),
        reason: VisibilityReason::ProductComponent,
    },
    Rule {
        tier: Tier::DisplayVocabulary,
        field: Field::Path,
        needles: SDK_SAMPLE_DIRECTORIES,
        outcome: Outcome::Weigh(-20),
        reason: VisibilityReason::SdkSample,
    },
    Rule {
        tier: Tier::VendorMetadata,
        field: Field::Prose,
        needles: SDK_SAMPLE_PROSE,
        outcome: Outcome::Weigh(-20),
        reason: VisibilityReason::SdkSample,
    },
    Rule {
        tier: Tier::DisplayVocabulary,
        field: Field::Path,
        needles: RUNTIME_DIRECTORIES,
        outcome: Outcome::Weigh(-20),
        reason: VisibilityReason::RuntimeDirectory,
    },
];

/// Generic words that name a component only when the file already sits somewhere components live.
///
/// Not a table entry because it is the one rule that needs corroboration: on their own these name
/// real products — `WindowsSandbox.exe` is Windows Sandbox itself, and a compiler is a developer
/// tool a user launches.
pub(in crate::catalog::visibility) fn is_corroborated_weak_component(
    fields: &MarkerFields,
) -> bool {
    fields.any(Field::FileName, &["compiler", "sandbox"])
        && fields.any(Field::Path, RUNTIME_DIRECTORIES)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mechanical form of "a word may not delete an application": the table cannot express
    /// rejection, so no future needle can reintroduce it by being added to the wrong list.
    #[test]
    fn no_text_rule_can_reject_a_record() {
        for rule in RULES {
            assert!(matches!(
                rule.outcome,
                Outcome::ForceAuxiliary(_) | Outcome::Weigh(_)
            ));
        }
    }

    /// Invariant I2: a needle written against one machine may nudge a borderline record, never
    /// settle it. The engine enforces it — a local rule cannot even express more than the cap.
    #[test]
    fn local_corpus_rules_cannot_settle_a_class() {
        for rule in RULES.iter().filter(|rule| rule.tier == Tier::LocalCorpus) {
            match rule.effect() {
                Outcome::Weigh(weight) => assert!(
                    weight >= -Tier::LOCAL_CORPUS_CAP,
                    "a local-corpus needle weighed {weight}"
                ),
                Outcome::ForceAuxiliary(_) => {
                    panic!("a local-corpus needle may only add weight")
                }
            }
        }
    }

    /// The cap is applied by `effect`, not by whoever wrote the table: a local rule declaring an
    /// ordinary marker weight still contributes only the capped amount.
    #[test]
    fn the_cap_is_applied_by_the_engine_not_by_the_table() {
        let overreaching = Rule {
            tier: Tier::LocalCorpus,
            field: Field::FileName,
            needles: &["whatever"],
            outcome: Outcome::Weigh(-80),
            reason: VisibilityReason::ProductComponent,
        };

        assert!(matches!(
            overreaching.effect(),
            Outcome::Weigh(weight) if weight == -Tier::LOCAL_CORPUS_CAP
        ));
    }

    /// A display name is localized and user-editable, so it may settle a record only into a list
    /// the user can see and undo — never below that.
    #[test]
    fn display_vocabulary_never_does_more_than_force_auxiliary() {
        for rule in RULES
            .iter()
            .filter(|rule| rule.tier == Tier::DisplayVocabulary)
        {
            match rule.outcome {
                Outcome::ForceAuxiliary(score) => assert!(score < 0),
                Outcome::Weigh(_) => {}
            }
        }
    }

    #[test]
    fn every_rule_carries_at_least_one_needle() {
        for rule in RULES {
            assert!(!rule.needles.is_empty());
        }
    }
}
