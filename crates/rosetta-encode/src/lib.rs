//! Surface encodings and the DSL parser for TPT Rosetta.
//!
//! - [`dsl`] parses and renders the Rosetta DSL (see `docs/notation.md`).
//! - [`render`] produces the English, concise English, structured and DSL
//!   encodings of a canonical task.
//! - [`encode_or_fallback`] applies the safe-fallback rule: when a task is
//!   refused, the original request is returned instead of a rewritten task.

pub mod dsl;
pub mod integration;
pub mod render;

use rosetta_core::{Diagnostic, Readiness, Task, diff, readiness, validate};

pub use render::{Encoding, decode, render};

/// The result of [`encode_or_fallback`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The task was encoded. `readiness` may still require clarification.
    Encoded {
        text: String,
        readiness: Readiness,
        diagnostics: Vec<Diagnostic>,
    },
    /// The task was refused, so the original request is returned unchanged.
    Fallback {
        original: String,
        diagnostics: Vec<Diagnostic>,
    },
    /// The task was refused and no original request was kept, so nothing can
    /// be sent in its place.
    Rejected { diagnostics: Vec<Diagnostic> },
}

/// Encodes `task`, or falls back to its original request when it is refused.
///
/// A refused task is never sent as a rewritten encoding. Tasks that need
/// clarification are still encoded, and the caller can read the readiness.
pub fn encode_or_fallback(task: &Task, encoding: Encoding) -> Outcome {
    let diagnostics = validate(task);
    match readiness(&diagnostics) {
        Readiness::Refused => match &task.source {
            Some(original) => Outcome::Fallback {
                original: original.clone(),
                diagnostics,
            },
            None => Outcome::Rejected { diagnostics },
        },
        ready => Outcome::Encoded {
            text: render(task, encoding),
            readiness: ready,
            diagnostics,
        },
    }
}

/// Checks that rendering `task` in `encoding` and decoding it again gives the
/// same canonical task. Only decodable encodings can be checked. Returns the
/// list of semantic differences, which is empty on success.
pub fn round_trip_differences(task: &Task, encoding: Encoding) -> Result<Vec<String>, String> {
    if !encoding.is_decodable() {
        return Err(format!("{encoding} is not decodable"));
    }
    let text = render(task, encoding);
    let decoded = decode(&text).map_err(|e| e.to_string())?;
    Ok(diff(task, &decoded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rosetta_core::{Permission, Polarity, Requirement, Strength};

    fn complete() -> Task {
        let mut task = Task::new("complete", "Add a regression test.");
        task.acceptance.push("cargo test passes".into());
        task.permissions.modify_files = Permission::Permitted;
        task.permissions.commit_changes = Permission::Forbidden;
        task.permissions.run_commands = Permission::Permitted;
        task
    }

    #[test]
    fn complete_task_is_encoded() {
        let outcome = encode_or_fallback(&complete(), Encoding::English);
        assert!(matches!(
            outcome,
            Outcome::Encoded {
                readiness: Readiness::Actionable,
                ..
            }
        ));
    }

    #[test]
    fn ambiguous_task_is_encoded_but_flagged() {
        let mut task = complete();
        task.unknowns.push("which module?".into());
        match encode_or_fallback(&task, Encoding::Concise) {
            Outcome::Encoded { readiness, .. } => {
                assert_eq!(readiness, Readiness::NeedsClarification)
            }
            other => panic!("expected encoded outcome, got {other:?}"),
        }
    }

    #[test]
    fn refused_task_falls_back_to_original_request() {
        let mut task = complete();
        task.source = Some("Keep the API and also change it.".into());
        task.requirements.push(Requirement {
            strength: Strength::Hard,
            polarity: Polarity::Positive,
            text: "keep the API".into(),
        });
        task.requirements.push(Requirement {
            strength: Strength::Hard,
            polarity: Polarity::Negative,
            text: "keep the API".into(),
        });
        match encode_or_fallback(&task, Encoding::Dsl) {
            Outcome::Fallback { original, .. } => {
                assert_eq!(original, "Keep the API and also change it.")
            }
            other => panic!("expected fallback, got {other:?}"),
        }
    }

    #[test]
    fn refused_task_without_source_is_rejected() {
        let mut task = complete();
        task.id = "Bad Id".into();
        assert!(matches!(
            encode_or_fallback(&task, Encoding::Dsl),
            Outcome::Rejected { .. }
        ));
    }

    #[test]
    fn dsl_round_trip_has_no_differences() {
        let mut task = complete();
        task.scope = vec!["crate:a".into()];
        task.requirements.push(Requirement {
            strength: Strength::Soft,
            polarity: Polarity::Negative,
            text: "add dependencies".into(),
        });
        assert_eq!(round_trip_differences(&task, Encoding::Dsl), Ok(vec![]));
    }

    #[test]
    fn non_decodable_encoding_cannot_round_trip() {
        assert!(round_trip_differences(&complete(), Encoding::English).is_err());
    }
}
