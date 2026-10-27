//! Run planning: which prompts each encoding produces for each task.
//!
//! Every encoding of a task is rendered from the same canonical task, so the
//! encodings are equivalent in requirements, permissions and acceptance
//! criteria. Refused tasks use their original request, as the safe fallback
//! requires. Planning makes no model calls.

use rosetta_encode::{Encoding, Outcome, encode_or_fallback};

use crate::corpus::{CorpusTask, Expect, Split};
use crate::tokens::{TokenEstimate, estimate_tokens};

/// One task rendered in one encoding, ready to send.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedRun {
    pub task_id: String,
    pub category: String,
    pub split: Split,
    pub expect: Expect,
    pub encoding: Encoding,
    /// `true` when the prompt is the original request, not an encoding.
    pub fallback: bool,
    pub prompt: String,
    pub estimated_input: TokenEstimate,
}

/// Plans one run per (task, encoding) pair, in corpus order.
pub fn plan(corpus: &[CorpusTask], encodings: &[Encoding]) -> Vec<PlannedRun> {
    let mut runs = Vec::new();
    for item in corpus {
        for &encoding in encodings {
            let (prompt, fallback) = match encode_or_fallback(&item.task, encoding) {
                Outcome::Encoded { text, .. } => (text, false),
                Outcome::Fallback { original, .. } => (original, true),
                Outcome::Rejected { .. } => (String::new(), true),
            };
            runs.push(PlannedRun {
                task_id: item.task.id.clone(),
                category: item.category.clone(),
                split: item.split,
                expect: item.expect,
                encoding,
                fallback,
                estimated_input: estimate_tokens(&prompt),
                prompt,
            });
        }
    }
    runs
}

/// Counts the planned runs that would be fallbacks, for reporting.
pub fn fallback_count(runs: &[PlannedRun]) -> usize {
    runs.iter().filter(|run| run.fallback).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::load_dir;
    use std::path::Path;

    fn shipped() -> Vec<CorpusTask> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/corpus/v1");
        load_dir(&dir).unwrap()
    }

    #[test]
    fn plans_one_run_per_task_and_encoding() {
        let corpus = shipped();
        let runs = plan(&corpus, &Encoding::ALL);
        assert_eq!(runs.len(), corpus.len() * Encoding::ALL.len());
    }

    #[test]
    fn refused_tasks_fall_back_to_original_request() {
        let corpus = shipped();
        let runs = plan(&corpus, &[Encoding::English]);
        for run in runs.iter().filter(|r| r.expect == Expect::Refuse) {
            assert!(run.fallback, "{} should fall back", run.task_id);
        }
        for run in runs.iter().filter(|r| r.expect == Expect::Actionable) {
            assert!(!run.fallback, "{} should be encoded", run.task_id);
        }
    }

    #[test]
    fn estimates_are_marked_as_estimates() {
        let corpus = shipped();
        for run in plan(&corpus, &Encoding::ALL) {
            assert!(!run.estimated_input.exact);
        }
    }

    #[test]
    fn injected_tasks_keep_their_stated_permissions() {
        // The injected text itself is copied into the objective as data. The
        // permission lines are what must stay unchanged.
        let corpus = shipped();
        let injected: Vec<_> = corpus
            .iter()
            .filter(|item| item.category == "injection-adversarial")
            .collect();
        assert!(
            !injected.is_empty(),
            "corpus should contain injection tasks"
        );
        for item in injected {
            let expected = item.task.permissions.commit_changes;
            for run in plan(
                std::slice::from_ref(item),
                &[Encoding::English, Encoding::Concise],
            ) {
                if run.fallback {
                    continue;
                }
                let granted = match expected {
                    rosetta_core::Permission::Forbidden => "forbidden",
                    rosetta_core::Permission::Permitted => "permitted",
                    rosetta_core::Permission::Unspecified => "not stated",
                };
                let line = match run.encoding {
                    Encoding::English => format!("- commit changes: {granted}"),
                    _ => format!("commit_changes={granted}"),
                };
                assert!(
                    run.prompt.contains(&line),
                    "{} lacks {line:?} in {}",
                    run.task_id,
                    run.encoding
                );
            }
        }
    }
}
