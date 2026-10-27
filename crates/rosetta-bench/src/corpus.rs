//! Loading and checking the labelled task corpus.
//!
//! A corpus file holds several DSL documents separated by `---` lines. Each
//! document starts with metadata comments:
//!
//! ```text
//! # category: rust-coding
//! # expect: actionable
//! R1 example-task
//! OBJ "..."
//! ```
//!
//! `expect` is the readiness the validator must return for the task. The
//! `dev` and `heldout` split is taken from the file name.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use rosetta_core::{Readiness, Task, readiness, validate};
use rosetta_encode::dsl;

/// The expected readiness of a corpus task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    Actionable,
    Clarify,
    Refuse,
}

impl Expect {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "actionable" => Ok(Expect::Actionable),
            "clarify" => Ok(Expect::Clarify),
            "refuse" => Ok(Expect::Refuse),
            other => Err(format!("unknown expect value {other:?}")),
        }
    }

    /// The readiness the validator must produce for this label.
    pub fn readiness(self) -> Readiness {
        match self {
            Expect::Actionable => Readiness::Actionable,
            Expect::Clarify => Readiness::NeedsClarification,
            Expect::Refuse => Readiness::Refused,
        }
    }
}

/// Which held-out split a task belongs to. Held-out tasks are kept out of
/// prompt and encoder tuning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    Dev,
    Heldout,
}

/// One labelled corpus task.
#[derive(Debug, Clone, PartialEq)]
pub struct CorpusTask {
    pub task: Task,
    pub category: String,
    pub expect: Expect,
    pub split: Split,
    /// File the task was loaded from, for error messages.
    pub file: String,
}

/// Loads every `.rosetta` file in `dir`, sorted by name.
pub fn load_dir(dir: &Path) -> Result<Vec<CorpusTask>, String> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rosetta"))
        .collect();
    paths.sort();

    let mut out = Vec::new();
    for path in paths {
        out.extend(load_file(&path)?);
    }
    Ok(out)
}

/// Loads one corpus file. The split comes from the file stem: `heldout` is
/// held-out, anything else is dev.
pub fn load_file(path: &Path) -> Result<Vec<CorpusTask>, String> {
    let source =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file = path.display().to_string();
    let split = if path.file_stem().is_some_and(|stem| stem == "heldout") {
        Split::Heldout
    } else {
        Split::Dev
    };

    let mut out = Vec::new();
    for (index, document) in dsl::split_documents(&source).into_iter().enumerate() {
        let (category, expect) =
            metadata(document).map_err(|e| format!("{file}, document {}: {e}", index + 1))?;
        let task =
            dsl::parse(document).map_err(|e| format!("{file}, document {}: {e}", index + 1))?;
        out.push(CorpusTask {
            task,
            category,
            expect,
            split,
            file: file.clone(),
        });
    }
    Ok(out)
}

fn metadata(document: &str) -> Result<(String, Expect), String> {
    let mut category = None;
    let mut expect = None;
    for line in document.lines() {
        let Some(comment) = line.trim().strip_prefix('#') else {
            continue;
        };
        let Some((key, value)) = comment.split_once(':') else {
            continue;
        };
        match key.trim() {
            "category" => category = Some(value.trim().to_string()),
            "expect" => expect = Some(Expect::parse(value.trim())?),
            _ => {}
        }
    }
    match (category, expect) {
        (Some(category), Some(expect)) => Ok((category, expect)),
        (None, _) => Err("missing `# category:` metadata".to_string()),
        (_, None) => Err("missing `# expect:` metadata".to_string()),
    }
}

/// Checks that every task's readiness matches its label and that task ids are
/// unique. Returns one message per problem. An empty result means the corpus
/// is consistent.
pub fn check(corpus: &[CorpusTask]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut ids = BTreeSet::new();

    for item in corpus {
        if !ids.insert(item.task.id.clone()) {
            problems.push(format!(
                "{}: duplicate task id {:?}",
                item.file, item.task.id
            ));
        }
        let actual = readiness(&validate(&item.task));
        if actual != item.expect.readiness() {
            problems.push(format!(
                "{}: task {:?} expected {:?} but validator returned {:?}",
                item.file,
                item.task.id,
                item.expect.readiness(),
                actual
            ));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_read_from_comments() {
        let document = "# category: rust-coding\n# expect: clarify\nR1 t\nOBJ \"x\"\n";
        let (category, expect) = metadata(document).unwrap();
        assert_eq!(category, "rust-coding");
        assert_eq!(expect, Expect::Clarify);
    }

    #[test]
    fn missing_expect_is_an_error() {
        assert!(metadata("# category: x\nR1 t\n").is_err());
    }

    #[test]
    fn unknown_expect_is_an_error() {
        assert!(metadata("# category: x\n# expect: maybe\n").is_err());
    }

    #[test]
    fn check_reports_mismatched_expectation() {
        let document = "# category: x\n# expect: actionable\nR1 t\nOBJ \"x\"\n";
        let task = dsl::parse(document).unwrap();
        let corpus = vec![CorpusTask {
            task,
            category: "x".into(),
            expect: Expect::Actionable,
            split: Split::Dev,
            file: "test".into(),
        }];
        // No acceptance criteria and no permissions, so the validator asks for clarification.
        let problems = check(&corpus);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("NeedsClarification"));
    }

    #[test]
    fn shipped_corpus_is_consistent() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/corpus/v1");
        let corpus = load_dir(&dir).unwrap();
        assert_eq!(corpus.len(), 50, "corpus v1 should hold 50 tasks");
        let problems = check(&corpus);
        assert!(
            problems.is_empty(),
            "corpus problems:\n{}",
            problems.join("\n")
        );
    }

    #[test]
    fn shipped_corpus_has_both_splits_and_all_expectations() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/corpus/v1");
        let corpus = load_dir(&dir).unwrap();
        assert!(corpus.iter().any(|t| t.split == Split::Dev));
        assert!(corpus.iter().any(|t| t.split == Split::Heldout));
        for expect in [Expect::Actionable, Expect::Clarify, Expect::Refuse] {
            assert!(
                corpus.iter().any(|t| t.expect == expect),
                "no {expect:?} tasks"
            );
        }
    }
}
