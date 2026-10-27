//! Canonical, model-independent semantic task model for TPT Rosetta.
//!
//! This crate defines the typed representation of a task, deterministic
//! canonicalisation, structural and semantic-readiness validation, and a
//! field-level diff. It has no dependencies and performs no I/O or network
//! access. Surface encodings live in `rosetta-encode`.

use std::fmt;

use unicode_normalization::{UnicodeNormalization, is_nfc};

pub mod confusables;

/// Schema version implemented by this crate.
pub const SCHEMA_VERSION: u32 = 1;

/// A canonical task.
///
/// Every field is explicit. Missing values stay missing: an absent permission
/// is [`Permission::Unspecified`], not a default grant or denial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// Stable task identifier. See [`is_valid_task_id`].
    pub id: String,
    /// Schema version the task was written against.
    pub schema_version: u32,
    /// The requested outcome.
    pub objective: String,
    /// Target resources, such as a crate, file or repository.
    pub scope: Vec<String>,
    /// Hard and soft constraints, positive and negative.
    pub requirements: Vec<Requirement>,
    /// Actions or artefacts to produce, in order.
    pub deliverables: Vec<String>,
    /// Identifiers of context the host supplies. These never grant permission.
    pub context: Vec<String>,
    /// Preconditions and ordering dependencies.
    pub dependencies: Vec<String>,
    /// Statements the requester asserts as known.
    pub facts: Vec<String>,
    /// Statements taken as true without verification.
    pub assumptions: Vec<String>,
    /// Open questions or missing values.
    pub unknowns: Vec<String>,
    /// Conditions a correct result satisfies, in order.
    pub acceptance: Vec<String>,
    /// Evidence that must be produced.
    pub evidence: Vec<String>,
    /// Execution and modification permissions.
    pub permissions: Permissions,
    /// Optional resource limits.
    pub budget: Budget,
    /// The original natural-language request, kept for audit and fallback.
    pub source: Option<String>,
}

impl Task {
    /// Creates an empty task with the current schema version.
    pub fn new(id: impl Into<String>, objective: impl Into<String>) -> Self {
        Task {
            id: id.into(),
            schema_version: SCHEMA_VERSION,
            objective: objective.into(),
            scope: Vec::new(),
            requirements: Vec::new(),
            deliverables: Vec::new(),
            context: Vec::new(),
            dependencies: Vec::new(),
            facts: Vec::new(),
            assumptions: Vec::new(),
            unknowns: Vec::new(),
            acceptance: Vec::new(),
            evidence: Vec::new(),
            permissions: Permissions::default(),
            budget: Budget::default(),
            source: None,
        }
    }
}

/// A constraint or preference on the result.
///
/// Strength and polarity are independent and are never inferred or changed by
/// an encoder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Requirement {
    pub strength: Strength,
    pub polarity: Polarity,
    pub text: String,
}

/// Whether a requirement must be met or is only preferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strength {
    /// Must be satisfied.
    Hard,
    /// Preferred, and may be traded off.
    Soft,
}

/// Whether a requirement asks for something or forbids it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Polarity {
    /// The result must or should include the property.
    Positive,
    /// The result must or should not include the property.
    Negative,
}

/// The state of one permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Permission {
    Permitted,
    Forbidden,
    /// The requester did not say. This is a missing value, not a default.
    #[default]
    Unspecified,
}

/// Execution and modification permissions for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Permissions {
    pub modify_files: Permission,
    pub commit_changes: Permission,
    pub run_commands: Permission,
    pub network: Permission,
}

impl Permissions {
    /// Names accepted by [`Permissions::set`], in canonical order.
    pub const NAMES: [&'static str; 4] =
        ["modify_files", "commit_changes", "run_commands", "network"];

    /// Returns the value of a named permission.
    pub fn get(&self, name: &str) -> Option<Permission> {
        match name {
            "modify_files" => Some(self.modify_files),
            "commit_changes" => Some(self.commit_changes),
            "run_commands" => Some(self.run_commands),
            "network" => Some(self.network),
            _ => None,
        }
    }

    /// Sets a named permission. Returns `false` if the name is unknown.
    pub fn set(&mut self, name: &str, value: Permission) -> bool {
        let slot = match name {
            "modify_files" => &mut self.modify_files,
            "commit_changes" => &mut self.commit_changes,
            "run_commands" => &mut self.run_commands,
            "network" => &mut self.network,
            _ => return false,
        };
        *slot = value;
        true
    }
}

/// Optional resource limits. Limits are not targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Budget {
    pub tokens: Option<u64>,
    pub seconds: Option<u64>,
}

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The task is structurally or semantically invalid. Conversion is refused.
    Error,
    /// The task is valid but needs clarification before it is actionable.
    Warning,
}

/// A human-readable finding about a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable code, such as `E004`. Codes are never reused.
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{level}[{}]: {}", self.code, self.message)
    }
}

/// Whether a task can be acted on as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// No diagnostics. The task may be encoded and executed.
    Actionable,
    /// Valid, but at least one warning must be resolved first.
    NeedsClarification,
    /// At least one error. Conversion must not produce an actionable task.
    Refused,
}

/// Returns `true` if `id` is a valid task identifier: 1 to 64 characters from
/// `[a-z0-9_.-]`, starting with a lowercase letter or digit.
pub fn is_valid_task_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    if !(bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit()) {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-'))
}

/// Returns `true` if `reference` is a valid context reference: 1 to 256
/// characters from `[A-Za-z0-9_.:/-]`.
pub fn is_valid_reference(reference: &str) -> bool {
    !reference.is_empty()
        && reference.len() <= 256
        && reference
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'/' | b'-'))
}

/// Returns `true` for control characters other than tab, and for invisible or
/// bidirectional-override characters that can make text read differently from
/// what a model receives. Zero-width characters, byte-order marks and the
/// bidi embedding, override and isolate controls are included.
pub fn is_invisible_or_control(c: char) -> bool {
    (c.is_control() && c != '\t')
        || matches!(
            c as u32,
            0x200B..=0x200F | 0x202A..=0x202E | 0x2066..=0x2069 | 0xFEFF
        )
}

/// Every free-text field of a task, for scans that do not care about structure.
fn canonical_text(task: &Task) -> Vec<&str> {
    let mut out: Vec<&str> = vec![task.objective.as_str()];
    for list in [
        &task.scope,
        &task.deliverables,
        &task.dependencies,
        &task.facts,
        &task.assumptions,
        &task.unknowns,
        &task.acceptance,
        &task.evidence,
    ] {
        out.extend(list.iter().map(String::as_str));
    }
    out.extend(task.requirements.iter().map(|r| r.text.as_str()));
    out
}

/// Every free-text field, including the audit copy in `source`.
fn all_text(task: &Task) -> Vec<&str> {
    let mut out = canonical_text(task);
    if let Some(source) = &task.source {
        out.push(source);
    }
    out
}

/// Checks a task and returns every finding, errors first.
///
/// Errors are structural or contradictory. Warnings mark missing essentials
/// and unresolved questions. Nothing is changed or inferred.
pub fn validate(task: &Task) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    if !is_valid_task_id(&task.id) {
        out.push(error("E001", format!("invalid task id {:?}", task.id)));
    }
    if task.schema_version != SCHEMA_VERSION {
        out.push(error(
            "E002",
            format!(
                "unsupported schema version {} (this build supports {SCHEMA_VERSION})",
                task.schema_version
            ),
        ));
    }
    if task.objective.trim().is_empty() {
        out.push(error("E003", "objective is empty".to_string()));
    }

    let lists: [(&str, &[String]); 8] = [
        ("scope", &task.scope),
        ("deliverables", &task.deliverables),
        ("dependencies", &task.dependencies),
        ("facts", &task.facts),
        ("assumptions", &task.assumptions),
        ("unknowns", &task.unknowns),
        ("acceptance", &task.acceptance),
        ("evidence", &task.evidence),
    ];
    for (field, items) in lists {
        for item in items {
            if item.trim().is_empty() {
                out.push(error("E005", format!("empty entry in {field}")));
            }
        }
    }
    for requirement in &task.requirements {
        if requirement.text.trim().is_empty() {
            out.push(error("E005", "empty requirement text".to_string()));
        }
    }

    for reference in &task.context {
        if !is_valid_reference(reference) {
            out.push(error(
                "E006",
                format!("invalid context reference {reference:?}"),
            ));
        }
    }
    for (index, reference) in task.context.iter().enumerate() {
        if task.context[..index].contains(reference) {
            out.push(warning(
                "W006",
                format!("duplicate context reference {reference:?}"),
            ));
        }
    }

    for (index, first) in task.requirements.iter().enumerate() {
        for second in &task.requirements[index + 1..] {
            if first.text != second.text || first.polarity == second.polarity {
                continue;
            }
            let message = format!(
                "{} and {} both apply to {:?}",
                label(first),
                label(second),
                first.text
            );
            match first.strength {
                Strength::Hard if second.strength == Strength::Hard => {
                    out.push(error("E004", format!("conflicting constraints: {message}")));
                }
                Strength::Soft if second.strength == Strength::Soft => {
                    out.push(warning(
                        "W005",
                        format!("conflicting preferences: {message}"),
                    ));
                }
                _ => {}
            }
        }
    }

    if task.acceptance.is_empty() {
        out.push(warning("W001", "no acceptance criteria".to_string()));
    }
    if !task.deliverables.is_empty() {
        for name in ["modify_files", "commit_changes"] {
            if task.permissions.get(name) == Some(Permission::Unspecified) {
                out.push(warning(
                    "W002",
                    format!("{name} is unspecified but the task has deliverables"),
                ));
            }
        }
    }
    if task.permissions.run_commands == Permission::Unspecified {
        out.push(warning(
            "W007",
            "run_commands is unspecified; state whether commands may run".to_string(),
        ));
    }
    for unknown in &task.unknowns {
        out.push(warning("W004", format!("unresolved question: {unknown}")));
    }
    for text in all_text(task) {
        if text.chars().any(is_invisible_or_control) {
            out.push(warning(
                "W008",
                format!("text contains invisible or control characters: {text:?}"),
            ));
            break;
        }
    }
    if canonical_text(task).iter().any(|text| !is_nfc(text)) {
        out.push(warning(
            "W009",
            "text is not in Unicode NFC; canonicalisation changes its bytes".to_string(),
        ));
    }
    let mut mixed: Vec<&str> = canonical_text(task)
        .iter()
        .flat_map(|text| confusables::mixed_script_words(text))
        .collect();
    mixed.sort_unstable();
    mixed.dedup();
    if !mixed.is_empty() {
        out.push(warning(
            "W010",
            format!("words mix Latin with another script: {mixed:?}"),
        ));
    }

    out.sort_by_key(|d| d.severity);
    out
}

/// Summarises diagnostics into a [`Readiness`] verdict.
///
/// Any error refuses the task. Any warning requires clarification. Warnings
/// are never treated as passed. v1 has only this strict mode.
pub fn readiness(diagnostics: &[Diagnostic]) -> Readiness {
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        Readiness::Refused
    } else if diagnostics.iter().any(|d| d.severity == Severity::Warning) {
        Readiness::NeedsClarification
    } else {
        Readiness::Actionable
    }
}

/// Returns a deterministic canonical form of `task`.
///
/// - Text is trimmed and runs of whitespace are collapsed to one space.
/// - Sets (scope, context, dependencies, facts, assumptions, unknowns,
///   evidence and requirements) are sorted and de-duplicated.
/// - Ordered lists (deliverables, acceptance) keep their order and lose only
///   repeated entries.
/// - `source` is kept exactly, because it is the audit record.
///
/// Canonicalisation never changes a requirement's strength or polarity, and
/// it is idempotent.
pub fn canonicalise(task: &Task) -> Task {
    let mut out = task.clone();
    out.objective = normalise_text(&task.objective);

    out.scope = sorted_set(&task.scope);
    out.context = sorted_set(&task.context);
    out.dependencies = sorted_set(&task.dependencies);
    out.facts = sorted_set(&task.facts);
    out.assumptions = sorted_set(&task.assumptions);
    out.unknowns = sorted_set(&task.unknowns);
    out.evidence = sorted_set(&task.evidence);
    out.deliverables = ordered_unique(&task.deliverables);
    out.acceptance = ordered_unique(&task.acceptance);

    let mut requirements: Vec<Requirement> = task
        .requirements
        .iter()
        .map(|r| Requirement {
            strength: r.strength,
            polarity: r.polarity,
            text: normalise_text(&r.text),
        })
        .collect();
    requirements.sort();
    requirements.dedup();
    out.requirements = requirements;

    out
}

/// Lists the semantic differences between two tasks, after canonicalising both.
///
/// An empty result means the tasks are canonically equal. Each entry names the
/// field and the items that differ, so it can be shown to a reviewer.
pub fn diff(original: &Task, reconstructed: &Task) -> Vec<String> {
    let a = canonicalise(original);
    let b = canonicalise(reconstructed);
    let mut out = Vec::new();

    if a.id != b.id {
        out.push(format!("id: {:?} != {:?}", a.id, b.id));
    }
    if a.schema_version != b.schema_version {
        out.push(format!(
            "schema_version: {} != {}",
            a.schema_version, b.schema_version
        ));
    }
    if a.objective != b.objective {
        out.push(format!("objective: {:?} != {:?}", a.objective, b.objective));
    }

    let requirement_labels = |task: &Task| -> Vec<String> {
        task.requirements
            .iter()
            .map(|r| format!("{}: {}", label(r), r.text))
            .collect()
    };
    push_list_diff(
        &mut out,
        "requirements",
        &requirement_labels(&a),
        &requirement_labels(&b),
    );

    let pairs: [(&str, &[String], &[String]); 8] = [
        ("scope", &a.scope, &b.scope),
        ("deliverables", &a.deliverables, &b.deliverables),
        ("context", &a.context, &b.context),
        ("dependencies", &a.dependencies, &b.dependencies),
        ("facts", &a.facts, &b.facts),
        ("assumptions", &a.assumptions, &b.assumptions),
        ("unknowns", &a.unknowns, &b.unknowns),
        ("acceptance", &a.acceptance, &b.acceptance),
    ];
    for (field, left, right) in pairs {
        push_list_diff(&mut out, field, left, right);
    }
    push_list_diff(&mut out, "evidence", &a.evidence, &b.evidence);

    for name in Permissions::NAMES {
        if a.permissions.get(name) != b.permissions.get(name) {
            out.push(format!(
                "{name}: {:?} != {:?}",
                a.permissions.get(name).unwrap_or_default(),
                b.permissions.get(name).unwrap_or_default()
            ));
        }
    }
    if a.budget != b.budget {
        out.push(format!("budget: {:?} != {:?}", a.budget, b.budget));
    }
    if a.source != b.source {
        out.push(format!("source: {:?} != {:?}", a.source, b.source));
    }

    out
}

fn error(code: &'static str, message: String) -> Diagnostic {
    Diagnostic {
        code,
        severity: Severity::Error,
        message,
    }
}

fn warning(code: &'static str, message: String) -> Diagnostic {
    Diagnostic {
        code,
        severity: Severity::Warning,
        message,
    }
}

/// Short label for a requirement, such as `MUST` or `SHOULD_NOT`.
pub fn label(requirement: &Requirement) -> &'static str {
    match (requirement.strength, requirement.polarity) {
        (Strength::Hard, Polarity::Positive) => "MUST",
        (Strength::Hard, Polarity::Negative) => "MUST_NOT",
        (Strength::Soft, Polarity::Positive) => "SHOULD",
        (Strength::Soft, Polarity::Negative) => "SHOULD_NOT",
    }
}

fn push_list_diff(out: &mut Vec<String>, field: &str, left: &[String], right: &[String]) {
    if left == right {
        return;
    }
    let only_left: Vec<&String> = left.iter().filter(|item| !right.contains(item)).collect();
    let only_right: Vec<&String> = right.iter().filter(|item| !left.contains(item)).collect();
    if only_left.is_empty() && only_right.is_empty() {
        out.push(format!("{field}: order differs"));
        return;
    }
    if !only_left.is_empty() {
        out.push(format!("{field}: only in original: {only_left:?}"));
    }
    if !only_right.is_empty() {
        out.push(format!("{field}: only in reconstructed: {only_right:?}"));
    }
}

fn normalise_text(text: &str) -> String {
    text.nfc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn sorted_set(items: &[String]) -> Vec<String> {
    let mut out: Vec<String> = items.iter().map(|s| normalise_text(s)).collect();
    out.sort();
    out.dedup();
    out
}

fn ordered_unique(items: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in items {
        let text = normalise_text(item);
        if !out.contains(&text) {
            out.push(text);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Task {
        let mut task = Task::new("fix-parser", "Fix the empty-input crash.");
        task.acceptance.push("cargo test passes".into());
        task.permissions.set("modify_files", Permission::Permitted);
        task.permissions
            .set("commit_changes", Permission::Forbidden);
        task.permissions.set("run_commands", Permission::Permitted);
        task
    }

    fn requirement(strength: Strength, polarity: Polarity, text: &str) -> Requirement {
        Requirement {
            strength,
            polarity,
            text: text.into(),
        }
    }

    #[test]
    fn complete_task_is_actionable() {
        let task = base();
        assert_eq!(readiness(&validate(&task)), Readiness::Actionable);
    }

    #[test]
    fn invalid_id_is_error() {
        let task = Task::new("Bad Id", "Do it");
        let codes: Vec<_> = validate(&task).iter().map(|d| d.code).collect();
        assert!(codes.contains(&"E001"));
    }

    #[test]
    fn unsupported_version_is_error() {
        let mut task = base();
        task.schema_version = 2;
        assert!(validate(&task).iter().any(|d| d.code == "E002"));
    }

    #[test]
    fn empty_objective_is_error() {
        let task = Task::new("empty", "   ");
        assert!(validate(&task).iter().any(|d| d.code == "E003"));
    }

    #[test]
    fn hard_contradiction_refuses() {
        let mut task = base();
        task.requirements.push(requirement(
            Strength::Hard,
            Polarity::Positive,
            "keep the API",
        ));
        task.requirements.push(requirement(
            Strength::Hard,
            Polarity::Negative,
            "keep the API",
        ));
        let diagnostics = validate(&task);
        assert!(diagnostics.iter().any(|d| d.code == "E004"));
        assert_eq!(readiness(&diagnostics), Readiness::Refused);
    }

    #[test]
    fn soft_contradiction_only_warns() {
        let mut task = base();
        task.requirements.push(requirement(
            Strength::Soft,
            Polarity::Positive,
            "small diff",
        ));
        task.requirements.push(requirement(
            Strength::Soft,
            Polarity::Negative,
            "small diff",
        ));
        let diagnostics = validate(&task);
        assert!(diagnostics.iter().any(|d| d.code == "W005"));
        assert!(!diagnostics.iter().any(|d| d.code == "E004"));
    }

    #[test]
    fn hard_and_soft_on_same_text_is_not_a_conflict() {
        let mut task = base();
        task.requirements
            .push(requirement(Strength::Hard, Polarity::Positive, "tests"));
        task.requirements
            .push(requirement(Strength::Soft, Polarity::Negative, "tests"));
        let diagnostics = validate(&task);
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.code == "E004" || d.code == "W005")
        );
    }

    #[test]
    fn missing_acceptance_needs_clarification() {
        let mut task = base();
        task.acceptance.clear();
        assert_eq!(readiness(&validate(&task)), Readiness::NeedsClarification);
    }

    #[test]
    fn unspecified_permission_with_deliverables_warns() {
        let mut task = base();
        task.permissions.modify_files = Permission::Unspecified;
        task.deliverables.push("add a regression test".into());
        assert!(validate(&task).iter().any(|d| d.code == "W002"));
    }

    #[test]
    fn unknowns_block_actionability() {
        let mut task = base();
        task.unknowns.push("which parser entry point?".into());
        let diagnostics = validate(&task);
        assert!(diagnostics.iter().any(|d| d.code == "W004"));
        assert_eq!(readiness(&diagnostics), Readiness::NeedsClarification);
    }

    #[test]
    fn invalid_context_reference_is_error() {
        let mut task = base();
        task.context.push("bad ref with spaces".into());
        assert!(validate(&task).iter().any(|d| d.code == "E006"));
    }

    #[test]
    fn errors_sort_before_warnings() {
        let mut task = Task::new("Bad", "");
        task.acceptance.clear();
        let diagnostics = validate(&task);
        let first_warning = diagnostics
            .iter()
            .position(|d| d.severity == Severity::Warning);
        let last_error = diagnostics
            .iter()
            .rposition(|d| d.severity == Severity::Error);
        assert!(last_error.unwrap() < first_warning.unwrap());
    }

    #[test]
    fn canonicalise_is_idempotent_and_deterministic() {
        let mut task = base();
        task.scope = vec!["  crate:b ".into(), "crate:a".into(), "crate:a".into()];
        task.requirements.push(requirement(
            Strength::Soft,
            Polarity::Negative,
            "  big   refactor ",
        ));
        task.requirements
            .push(requirement(Strength::Hard, Polarity::Positive, "tests"));
        let once = canonicalise(&task);
        let twice = canonicalise(&once);
        assert_eq!(once, twice);
        assert_eq!(
            once.scope,
            vec!["crate:a".to_string(), "crate:b".to_string()]
        );
        assert_eq!(once.requirements[0].strength, Strength::Hard);
        assert_eq!(once.requirements[1].text, "big refactor");
    }

    #[test]
    fn canonicalise_preserves_strength_and_polarity() {
        let mut task = base();
        task.requirements
            .push(requirement(Strength::Soft, Polarity::Negative, "no unsafe"));
        let canonical = canonicalise(&task);
        assert_eq!(canonical.requirements[0].strength, Strength::Soft);
        assert_eq!(canonical.requirements[0].polarity, Polarity::Negative);
    }

    #[test]
    fn canonicalise_keeps_source_exactly() {
        let mut task = base();
        task.source = Some("  Please   fix it \n".into());
        assert_eq!(canonicalise(&task).source, task.source);
    }

    #[test]
    fn diff_of_equal_tasks_is_empty() {
        let mut a = base();
        a.scope = vec!["b".into(), "a".into()];
        let mut b = base();
        b.scope = vec!["a".into(), "b".into()];
        assert!(diff(&a, &b).is_empty());
    }

    #[test]
    fn diff_reports_dropped_prohibition() {
        let mut original = base();
        original.requirements.push(requirement(
            Strength::Hard,
            Polarity::Negative,
            "use unsafe",
        ));
        let reconstructed = base();
        let differences = diff(&original, &reconstructed);
        assert!(
            differences
                .iter()
                .any(|d| d.starts_with("requirements:") && d.contains("MUST_NOT"))
        );
    }

    #[test]
    fn diff_reports_changed_permission() {
        let original = base();
        let mut reconstructed = base();
        reconstructed.permissions.commit_changes = Permission::Permitted;
        let differences = diff(&original, &reconstructed);
        assert_eq!(
            differences,
            vec!["commit_changes: Forbidden != Permitted".to_string()]
        );
    }

    #[test]
    fn unstated_run_commands_needs_clarification() {
        let mut task = base();
        task.permissions.run_commands = Permission::Unspecified;
        let diagnostics = validate(&task);
        assert!(diagnostics.iter().any(|d| d.code == "W007"));
        assert_eq!(readiness(&diagnostics), Readiness::NeedsClarification);
    }

    #[test]
    fn invisible_characters_in_text_need_clarification() {
        let mut task = base();
        task.deliverables.push("review\u{202e}the code".into());
        let diagnostics = validate(&task);
        assert!(diagnostics.iter().any(|d| d.code == "W008"));
        assert_eq!(readiness(&diagnostics), Readiness::NeedsClarification);
    }

    #[test]
    fn ordinary_text_has_no_invisible_character_warning() {
        let task = base();
        assert!(!validate(&task).iter().any(|d| d.code == "W008"));
    }

    #[test]
    fn canonicalisation_composes_to_nfc() {
        // "é" as U+0065 U+0301 (decomposed) and U+00E9 (precomposed).
        let decomposed = "cafe\u{301}";
        let precomposed = "caf\u{e9}";
        let mut a = base();
        a.objective = decomposed.into();
        let mut b = base();
        b.objective = precomposed.into();
        assert_eq!(canonicalise(&a), canonicalise(&b));
        assert_eq!(canonicalise(&a).objective, precomposed);
    }

    #[test]
    fn canonicalisation_keeps_source_bytes() {
        let mut task = base();
        task.source = Some("cafe\u{301}".into());
        assert_eq!(canonicalise(&task).source, task.source);
    }

    #[test]
    fn non_nfc_task_text_warns() {
        let mut task = base();
        task.objective = "cafe\u{301}".into();
        let diagnostics = validate(&task);
        assert!(diagnostics.iter().any(|d| d.code == "W009"));
        assert_eq!(readiness(&diagnostics), Readiness::NeedsClarification);
    }

    #[test]
    fn non_nfc_source_alone_does_not_warn() {
        let mut task = base();
        task.source = Some("cafe\u{301}".into());
        assert!(!validate(&task).iter().any(|d| d.code == "W009"));
    }

    #[test]
    fn mixed_script_word_needs_clarification() {
        let mut task = base();
        task.objective = "Add a link to p\u{430}ypal.com in the footer.".into();
        let diagnostics = validate(&task);
        assert!(diagnostics.iter().any(|d| d.code == "W010"));
        assert_eq!(readiness(&diagnostics), Readiness::NeedsClarification);
    }

    #[test]
    fn ordinary_latin_text_has_no_mixed_script_warning() {
        let task = base();
        assert!(!validate(&task).iter().any(|d| d.code == "W010"));
    }

    #[test]
    fn permission_default_is_unspecified() {
        assert_eq!(Permissions::default().network, Permission::Unspecified);
    }

    #[test]
    fn set_rejects_unknown_permission_name() {
        let mut permissions = Permissions::default();
        assert!(!permissions.set("root_access", Permission::Permitted));
    }

    #[test]
    fn task_id_rules() {
        assert!(is_valid_task_id("dev-01"));
        assert!(is_valid_task_id("a.b_c-9"));
        assert!(!is_valid_task_id("-leading"));
        assert!(!is_valid_task_id("Upper"));
        assert!(!is_valid_task_id(""));
        assert!(!is_valid_task_id(&"a".repeat(65)));
    }
}
