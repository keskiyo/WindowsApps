use super::super::VisibilityReason;
use crate::catalog::fields::{Field, MarkerFields};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::catalog::visibility) enum Tier {
    VendorMetadata,
    DisplayVocabulary,
    LocalCorpus,
}

impl Tier {
    const LOCAL_CORPUS_CAP: i16 = 10;

    fn cap(self, weight: i16) -> i16 {
        match self {
            Self::LocalCorpus => weight.max(-Self::LOCAL_CORPUS_CAP),
            Self::VendorMetadata | Self::DisplayVocabulary => weight,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::catalog::visibility) enum Outcome {
    ForceAuxiliary(i16),
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
    pub(in crate::catalog::visibility) fn effect(&self) -> Outcome {
        match self.outcome {
            Outcome::ForceAuxiliary(score) => Outcome::ForceAuxiliary(score),
            Outcome::Weigh(weight) => Outcome::Weigh(self.tier.cap(weight)),
        }
    }
}

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
    "openconsole.exe",
];

const MAINTENANCE_NAMES: &[&str] = &[
    "crash handler",
    "elevation service",
    "bug report",
    "background task host",
    "деинстал",
    "reset preferences",
    "reset cache",
    "reset config",
    "reset settings",
    "сброс настроек",
    "сброс кэш",
];

const COMPONENT_FILE_NAMES: &[&str] = &[
    "crashpad_handler",
    "crash_reporter",
    "crashreporter",
    "dpinst.exe",
    "broker.exe",
    "pwa_launcher",
    "util.exe",
    "service.exe",
    "_helper",
    "-helper",
    "helper.exe",
];

const COMPONENT_PROSE: &[&str] = &[
    "language server",
    "openjdk platform binary",
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
    " module",
];

const LOCAL_COMPONENT_FILE_NAMES: &[&str] = &["iconv.exe", "intelliphp.ls"];

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

pub(in crate::catalog::visibility) fn is_corroborated_weak_component(
    fields: &MarkerFields,
) -> bool {
    fields.any(Field::FileName, &["compiler", "sandbox"])
        && fields.any(Field::Path, RUNTIME_DIRECTORIES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_text_rule_can_reject_a_record() {
        for rule in RULES {
            assert!(matches!(
                rule.outcome,
                Outcome::ForceAuxiliary(_) | Outcome::Weigh(_)
            ));
        }
    }

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
