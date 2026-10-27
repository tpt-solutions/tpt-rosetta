//! Benchmark harness for TPT Rosetta.
//!
//! The harness is offline by default. It loads a labelled task corpus, plans
//! the runs for each encoding, and summarises recorded trials. Live model calls
//! go through the [`ModelClient`] trait, which the host supplies. No provider
//! client lives in this crate, so no network access or cost is incurred here.

pub mod corpus;
pub mod metrics;
pub mod plan;
pub mod runner;
pub mod tokens;

pub use corpus::{CorpusTask, Expect, Split, load_dir, load_file};
pub use metrics::{Summary, Trial, summarise, summarise_by_category};
pub use plan::{PlannedRun, plan};
pub use runner::{Score, Scorer, run_plan};
pub use tokens::{TokenEstimate, estimate_tokens};

/// A completion returned by a host-supplied model client.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub text: String,
    /// Exact input tokens reported by the provider, if any.
    pub input_tokens: Option<u64>,
    /// Exact output tokens reported by the provider, if any.
    pub output_tokens: Option<u64>,
}

/// A host-supplied model. Implementations own credentials, routing and cost.
pub trait ModelClient {
    /// Name of the model profile, recorded with every trial.
    fn model_name(&self) -> &str;

    /// Sends one prompt. The caller is responsible for opt-in and cost
    /// disclosure before calling this.
    fn complete(&self, prompt: &str) -> Result<Completion, String>;
}
