//! Executes planned runs through a host-supplied [`ModelClient`].
//!
//! The runner does not decide whether a live call is allowed. The caller
//! chooses the client, so opt-in and cost disclosure happen before
//! [`run_plan`] is called. Success is decided by a host [`Scorer`]. Without a
//! scorer there is no default, so trials are never marked successful by
//! accident.

use std::time::Instant;

use crate::metrics::Trial;
use crate::plan::PlannedRun;
use crate::tokens::estimate_tokens;
use crate::{Completion, ModelClient};

/// The scored outcome of one completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Score {
    /// Every acceptance criterion was met.
    pub success: bool,
    /// Semantic constraint violations found by scoring.
    pub violations: u32,
}

/// Host-supplied scoring against a planned run's acceptance criteria.
pub trait Scorer {
    fn score(&self, run: &PlannedRun, completion: &str) -> Score;
}

/// Runs each planned run once and returns one trial per run.
///
/// A client error is recorded as a failed trial with zero output tokens. It is
/// not retried here, so the trial count always matches the plan.
pub fn run_plan<C: ModelClient, S: Scorer>(
    runs: &[PlannedRun],
    client: &C,
    scorer: &S,
) -> Vec<Trial> {
    runs.iter()
        .map(|run| run_one(run, client, scorer))
        .collect()
}

fn run_one<C: ModelClient, S: Scorer>(run: &PlannedRun, client: &C, scorer: &S) -> Trial {
    let started = Instant::now();
    let result = client.complete(&run.prompt);
    let latency_ms = started.elapsed().as_millis() as u64;

    match result {
        Ok(completion) => {
            trial_from_completion(run, client.model_name(), &completion, latency_ms, scorer)
        }
        Err(_) => Trial {
            task_id: run.task_id.clone(),
            category: run.category.clone(),
            encoding: run.encoding.to_string(),
            model: client.model_name().to_string(),
            input_tokens: run.estimated_input.tokens,
            output_tokens: 0,
            tokens_exact: false,
            latency_ms,
            success: false,
            violations: 0,
            retries: 0,
            cost_usd: None,
        },
    }
}

fn trial_from_completion<S: Scorer>(
    run: &PlannedRun,
    model: &str,
    completion: &Completion,
    latency_ms: u64,
    scorer: &S,
) -> Trial {
    let score = scorer.score(run, &completion.text);
    let input = completion.input_tokens;
    let output = completion.output_tokens;
    Trial {
        task_id: run.task_id.clone(),
        category: run.category.clone(),
        encoding: run.encoding.to_string(),
        model: model.to_string(),
        input_tokens: input.unwrap_or(run.estimated_input.tokens),
        output_tokens: output.unwrap_or_else(|| estimate_tokens(&completion.text).tokens),
        tokens_exact: input.is_some() && output.is_some(),
        latency_ms,
        success: score.success,
        violations: score.violations,
        retries: 0,
        cost_usd: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::{Expect, Split};
    use rosetta_encode::Encoding;

    struct Fake {
        reply: Result<Completion, String>,
    }

    impl ModelClient for Fake {
        fn model_name(&self) -> &str {
            "fake-model"
        }

        fn complete(&self, _prompt: &str) -> Result<Completion, String> {
            self.reply.clone()
        }
    }

    struct ContainsDone;

    impl Scorer for ContainsDone {
        fn score(&self, _run: &PlannedRun, completion: &str) -> Score {
            Score {
                success: completion.contains("done"),
                violations: 0,
            }
        }
    }

    fn run() -> PlannedRun {
        PlannedRun {
            task_id: "dev-01".into(),
            category: "bug-fixing".into(),
            split: Split::Dev,
            expect: Expect::Actionable,
            encoding: Encoding::Concise,
            fallback: false,
            prompt: "Goal: fix it".into(),
            estimated_input: estimate_tokens("Goal: fix it"),
        }
    }

    #[test]
    fn one_trial_per_planned_run() {
        let client = Fake {
            reply: Ok(Completion {
                text: "done".into(),
                input_tokens: Some(10),
                output_tokens: Some(3),
            }),
        };
        let trials = run_plan(&[run(), run()], &client, &ContainsDone);
        assert_eq!(trials.len(), 2);
        assert!(trials[0].success);
        assert_eq!(trials[0].model, "fake-model");
        assert_eq!(trials[0].encoding, "concise");
    }

    #[test]
    fn exact_counts_are_marked_exact() {
        let client = Fake {
            reply: Ok(Completion {
                text: "done".into(),
                input_tokens: Some(10),
                output_tokens: Some(3),
            }),
        };
        let trial = &run_plan(&[run()], &client, &ContainsDone)[0];
        assert!(trial.tokens_exact);
        assert_eq!(trial.input_tokens, 10);
        assert_eq!(trial.output_tokens, 3);
    }

    #[test]
    fn missing_counts_fall_back_to_marked_estimates() {
        let client = Fake {
            reply: Ok(Completion {
                text: "done".into(),
                input_tokens: None,
                output_tokens: None,
            }),
        };
        let trial = &run_plan(&[run()], &client, &ContainsDone)[0];
        assert!(!trial.tokens_exact);
        assert_eq!(trial.input_tokens, run().estimated_input.tokens);
    }

    #[test]
    fn client_error_is_a_failed_trial_not_dropped() {
        let client = Fake {
            reply: Err("upstream timeout".into()),
        };
        let trials = run_plan(&[run()], &client, &ContainsDone);
        assert_eq!(trials.len(), 1);
        assert!(!trials[0].success);
        assert_eq!(trials[0].cost_usd, None);
    }

    #[test]
    fn scorer_decides_success_not_the_runner() {
        let client = Fake {
            reply: Ok(Completion {
                text: "still working".into(),
                input_tokens: None,
                output_tokens: None,
            }),
        };
        assert!(!run_plan(&[run()], &client, &ContainsDone)[0].success);
    }
}
