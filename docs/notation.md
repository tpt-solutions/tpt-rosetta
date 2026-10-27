# Rosetta DSL notation reference

Version: DSL 1 (schema version 1). Status: experimental.

This document defines every keyword and field of the Rosetta DSL. The parser
in `crates/rosetta-encode` implements exactly this grammar. Anything not listed
here is a parse error.

## Document shape

A document describes one canonical task. It consists of:

1. Zero or more comment lines.
2. Exactly one header line: `R<version> <task-id>`.
3. Zero or more clause lines, in any order.

Blank lines are ignored. Comment lines start with `#` as the first character.

```
# category: rust-refactoring
R1 split-parser-module
OBJ "Split the parser module into lexer and grammar files."
MUST_NOT "Change the public API of rosetta-encode."
SHOULD "Keep each file under 400 lines."
DO "Move the tokenizer into lexer.rs."
ACCEPT "cargo test passes without changes to test files."
PERM modify_files=permitted commit_changes=forbidden
```

Multiple tasks may share one file. The corpus loader separates them with a line
containing only `---`.

## Header

| Form | Meaning |
| --- | --- |
| `R1 <id>` | Schema version 1, task identifier `<id>`. |
| `R<n> <id>` with `n` other than 1 | Rejected: unsupported schema version. |

Task identifiers match `[a-z0-9][a-z0-9_.-]{0,63}`. Identifiers are stable and
must not be reused for a different task.

## Values

- **Strings** are double-quoted. Escapes: `\"`, `\\`, `\n`, `\t`, `\u{HEX}`.
  Any other escape is an error. Raw control characters other than tab are
  rejected.
- **Reference identifiers** (`CTX`, `PERM` names) are bare tokens matching
  `[A-Za-z0-9_.:/-]{1,256}`, or a quoted string with the same content.
- **Integers** are unsigned decimal numbers.

## Clauses

### Objective and scope

| Clause | Arity | Field | Meaning |
| --- | --- | --- | --- |
| `OBJ "text"` | exactly 1 | objective | The requested outcome. Required. |
| `SCOPE "text"` | 0..n | scope | Target resources, such as a crate, file, or repository. |

### Constraints

Constraints carry two independent properties: **strength** (hard or soft) and
**polarity** (positive or negative).

| Clause | Strength | Polarity | Meaning |
| --- | --- | --- | --- |
| `MUST "text"` | hard | positive | The result must include the stated property. |
| `MUST_NOT "text"` | hard | negative | The result must not include the stated property. |
| `SHOULD "text"` | soft | positive | Preferred, but may be traded off. |
| `SHOULD_NOT "text"` | soft | negative | Avoided where practical. |

The strength is never inferred or changed. A `SHOULD` stays soft in every
encoding, and a `MUST_NOT` stays a prohibition in every encoding.

### Deliverables, context, and dependencies

| Clause | Arity | Field | Meaning |
| --- | --- | --- | --- |
| `DO "text"` | 0..n | deliverables | Actions or artefacts to produce. |
| `CTX ref` | 0..n | context references | Identifier of material the host supplies. A reference is not a permission. |
| `DEP "text"` | 0..n | dependencies | A precondition or ordering dependency. |

### Knowledge state

| Clause | Arity | Meaning |
| --- | --- | --- |
| `FACT "text"` | 0..n | A statement the requester asserts as known. |
| `ASSUME "text"` | 0..n | A statement taken as true without verification. |
| `UNKNOWN "text"` | 0..n | An open question or missing value. Any `UNKNOWN` makes a task need clarification. |

### Acceptance and evidence

| Clause | Arity | Meaning |
| --- | --- | --- | 
| `ACCEPT "text"` | 0..n | A condition that a correct result satisfies. |
| `EVIDENCE "text"` | 0..n | Evidence that must be produced, such as test output. |

### Permissions

`PERM name=value ...` sets permissions. Each name appears at most once per
clause, and the clause may appear once per document.

| Name | Values |
| --- | --- |
| `modify_files` | `permitted`, `forbidden`, `unspecified` |
| `commit_changes` | `permitted`, `forbidden`, `unspecified` |
| `run_commands` | `permitted`, `forbidden`, `unspecified` |
| `network` | `permitted`, `forbidden`, `unspecified` |

A field left out of `PERM` is **unspecified**, which is different from
`forbidden`. An unspecified permission is a missing value, not a default. The
validator reports it, and no encoding turns it into a grant.

### Budget

`BUDGET tokens=<n> seconds=<n>`. Either key may be omitted. Both are optional
limits, not targets.

### Provenance

`SRC "text"` records the original natural-language request. It is kept for
audit and for the fallback path. At most one `SRC` clause is allowed.

## Errors

The parser reports the line number and a message for each error. It never skips
an unknown clause or silently ignores a malformed value.

| Condition | Result |
| --- | --- |
| Unknown clause keyword | Error. |
| Missing or misplaced header | Error. |
| Unsupported schema version | Error. |
| More than one `OBJ` | Error. |
| Unterminated string or invalid escape | Error. |
| Invalid identifier or reference | Error. |
| Duplicate `PERM` name, or duplicate `SRC` | Error. |

Semantic problems (missing acceptance criteria, conflicting requirements) are
reported by the validator, not the parser. See `docs/security.md` for how each
diagnostic affects readiness.

## Keyword stability

Keywords are fixed in DSL 1. Adding a keyword requires a new DSL version. Adding
a new permission name or budget key also requires a new version, so older
parsers never meet fields they cannot read.
