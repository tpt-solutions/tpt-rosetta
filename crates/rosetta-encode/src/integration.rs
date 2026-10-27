//! Prototype of host-supplied context references.
//!
//! A host such as TPT Weave selects context and then hands Rosetta identifiers
//! for it. Rosetta stores only those identifiers as `CTX` references. It never
//! stores the content, and a reference never grants a permission. See
//! `docs/integrations.md`.

use std::collections::BTreeSet;

use rosetta_core::{Task, is_valid_reference};

/// Whether the host can supply the material behind a reference.
///
/// Rosetta cannot check availability itself. The host implements this trait,
/// and a reference the host cannot resolve must be reported to the model
/// rather than silently dropped.
pub trait ContextResolver {
    fn is_available(&self, reference: &str) -> bool;
}

/// Adds host-selected references to `task` as `CTX` entries.
///
/// References are validated and de-duplicated. Permissions, constraints and
/// every other field are left untouched. Returns the references that the
/// resolver cannot supply, so the caller can report them.
pub fn attach_context_references<R: ContextResolver>(
    task: &mut Task,
    references: &[String],
    resolver: &R,
) -> Result<Vec<String>, String> {
    let mut unavailable = Vec::new();
    for reference in references {
        if !is_valid_reference(reference) {
            return Err(format!("invalid context reference {reference:?}"));
        }
        if !resolver.is_available(reference) {
            unavailable.push(reference.clone());
        }
        if !task.context.contains(reference) {
            task.context.push(reference.clone());
        }
    }
    let unique: BTreeSet<_> = unavailable.iter().cloned().collect();
    Ok(unique.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rosetta_core::{Permission, diff};

    struct Available<'a>(&'a [&'a str]);

    impl ContextResolver for Available<'_> {
        fn is_available(&self, reference: &str) -> bool {
            self.0.contains(&reference)
        }
    }

    fn task() -> Task {
        let mut task = Task::new("with-context", "Fix the resampler.");
        task.permissions.modify_files = Permission::Permitted;
        task
    }

    #[test]
    fn references_are_added_without_changing_permissions() {
        let mut task = task();
        let before = task.clone();
        let refs = vec!["file:src/resample.rs".to_string()];
        let missing =
            attach_context_references(&mut task, &refs, &Available(&["file:src/resample.rs"]))
                .unwrap();
        assert!(missing.is_empty());
        assert_eq!(task.context, refs);
        assert!(
            diff(&before, &task)
                .iter()
                .all(|d| d.starts_with("context:"))
        );
    }

    #[test]
    fn unavailable_references_are_reported_not_dropped() {
        let mut task = task();
        let refs = vec!["file:a.rs".to_string(), "file:b.rs".to_string()];
        let missing =
            attach_context_references(&mut task, &refs, &Available(&["file:a.rs"])).unwrap();
        assert_eq!(missing, vec!["file:b.rs".to_string()]);
        assert_eq!(task.context.len(), 2);
    }

    #[test]
    fn duplicate_references_are_added_once() {
        let mut task = task();
        let refs = vec!["file:a.rs".to_string(), "file:a.rs".to_string()];
        attach_context_references(&mut task, &refs, &Available(&["file:a.rs"])).unwrap();
        assert_eq!(task.context, vec!["file:a.rs".to_string()]);
    }

    #[test]
    fn invalid_reference_is_rejected() {
        let mut task = task();
        let refs = vec!["has space".to_string()];
        assert!(attach_context_references(&mut task, &refs, &Available(&[])).is_err());
        assert!(task.context.is_empty());
    }
}
