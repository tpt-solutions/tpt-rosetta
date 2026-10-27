//! Token counting.
//!
//! No tokenizer is bundled, so counts here are estimates. Every estimate
//! carries its method and an `exact: false` flag, so an estimate is never
//! compared with an exact provider count as if they were equal. Exact counts
//! come from a host-supplied [`crate::Completion`] or a provider tokenizer.

/// A token count with the method that produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenEstimate {
    pub tokens: u64,
    /// Always `false` for this module. Exact counts are not produced here.
    pub exact: bool,
    pub method: &'static str,
}

/// Method name recorded with every estimate.
pub const ESTIMATE_METHOD: &str = "chars/4 heuristic (estimated, not a tokenizer count)";

/// Estimates tokens as one token per four Unicode scalar values, rounded up.
///
/// This is a rough heuristic for planning only. It is not a model's tokenizer,
/// and it can be far off for symbols, code, or non-English text.
pub fn estimate_tokens(text: &str) -> TokenEstimate {
    let chars = text.chars().count() as u64;
    TokenEstimate {
        tokens: chars.div_ceil(4),
        exact: false,
        method: ESTIMATE_METHOD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_rounds_up() {
        assert_eq!(estimate_tokens("").tokens, 0);
        assert_eq!(estimate_tokens("abcd").tokens, 1);
        assert_eq!(estimate_tokens("abcde").tokens, 2);
    }

    #[test]
    fn estimate_is_never_marked_exact() {
        let estimate = estimate_tokens("hello");
        assert!(!estimate.exact);
        assert_eq!(estimate.method, ESTIMATE_METHOD);
    }

    #[test]
    fn estimate_counts_scalars_not_bytes() {
        // Four scalars, twelve bytes.
        assert_eq!(estimate_tokens("日本語字").tokens, 1);
    }
}
