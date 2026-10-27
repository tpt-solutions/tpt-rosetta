//! Parser and renderer for the Rosetta DSL, version 1.
//!
//! The grammar is specified in `docs/notation.md`. Parsing is strict: unknown
//! clauses, malformed values and duplicate singleton fields are errors with a
//! line number. Nothing is skipped silently.

use std::fmt;

use rosetta_core::{
    Budget, Permission, Permissions, Polarity, Requirement, SCHEMA_VERSION, Strength, Task,
    is_invisible_or_control, is_valid_reference, is_valid_task_id,
};

/// A parse failure with the 1-based line where it occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Splits a corpus file into documents separated by lines that contain only
/// `---`. Empty documents are dropped.
pub fn split_documents(source: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            push_document(&mut out, &source[start..offset]);
            start = offset + line.len();
        }
        offset += line.len();
    }
    push_document(&mut out, &source[start..]);
    out
}

fn push_document<'a>(out: &mut Vec<&'a str>, document: &'a str) {
    if !document.trim().is_empty() {
        out.push(document);
    }
}

/// Parses one DSL document into a [`Task`].
pub fn parse(source: &str) -> Result<Task, ParseError> {
    let mut task: Option<Task> = None;
    let mut seen = Seen::default();

    for (index, raw) in source.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fail = |message: String| ParseError {
            line: line_no,
            message,
        };

        match task.as_mut() {
            None => task = Some(parse_header(line).map_err(fail)?),
            Some(current) => parse_clause(current, &mut seen, line).map_err(fail)?,
        }
    }

    task.ok_or(ParseError {
        line: 0,
        message: "missing header line (expected `R1 <task-id>`)".to_string(),
    })
}

/// Renders a task as a DSL document. For any task whose strings are valid
/// UTF-8, `parse(&render(task))` returns the same task.
pub fn render(task: &Task) -> String {
    let mut out = String::new();
    out.push_str(&format!("R{} {}\n", task.schema_version, task.id));
    out.push_str(&format!("OBJ {}\n", quote(&task.objective)));
    for scope in &task.scope {
        out.push_str(&format!("SCOPE {}\n", quote(scope)));
    }
    for requirement in &task.requirements {
        out.push_str(&format!(
            "{} {}\n",
            requirement_keyword(requirement),
            quote(&requirement.text)
        ));
    }
    for deliverable in &task.deliverables {
        out.push_str(&format!("DO {}\n", quote(deliverable)));
    }
    for context in &task.context {
        out.push_str(&format!("CTX {}\n", context));
    }
    for dependency in &task.dependencies {
        out.push_str(&format!("DEP {}\n", quote(dependency)));
    }
    for fact in &task.facts {
        out.push_str(&format!("FACT {}\n", quote(fact)));
    }
    for assumption in &task.assumptions {
        out.push_str(&format!("ASSUME {}\n", quote(assumption)));
    }
    for unknown in &task.unknowns {
        out.push_str(&format!("UNKNOWN {}\n", quote(unknown)));
    }
    for criterion in &task.acceptance {
        out.push_str(&format!("ACCEPT {}\n", quote(criterion)));
    }
    for evidence in &task.evidence {
        out.push_str(&format!("EVIDENCE {}\n", quote(evidence)));
    }
    out.push_str(&format!("PERM {}\n", render_permissions(task)));
    if task.budget.tokens.is_some() || task.budget.seconds.is_some() {
        out.push_str(&format!("BUDGET {}\n", render_budget(&task.budget)));
    }
    if let Some(source) = &task.source {
        out.push_str(&format!("SRC {}\n", quote(source)));
    }
    out
}

/// Escapes and quotes a string for the DSL.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if is_invisible_or_control(c) => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Tracks singleton clauses so duplicates are reported.
#[derive(Default)]
struct Seen {
    objective: bool,
    permissions: bool,
    budget: bool,
    source: bool,
}

fn parse_header(line: &str) -> Result<Task, String> {
    let mut parts = line.split_whitespace();
    let version = parts
        .next()
        .and_then(|token| token.strip_prefix('R'))
        .ok_or_else(|| "expected header `R1 <task-id>`".to_string())?;
    let version: u32 = version
        .parse()
        .map_err(|_| format!("invalid schema version {version:?}"))?;
    if version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema version {version} (this build supports {SCHEMA_VERSION})"
        ));
    }
    let id = parts
        .next()
        .ok_or_else(|| "header is missing a task id".to_string())?;
    if parts.next().is_some() {
        return Err("header has unexpected text after the task id".to_string());
    }
    if !is_valid_task_id(id) {
        return Err(format!("invalid task id {id:?}"));
    }
    Ok(Task::new(id, ""))
}

fn parse_clause(task: &mut Task, seen: &mut Seen, line: &str) -> Result<(), String> {
    let (keyword, rest) = match line.find(char::is_whitespace) {
        Some(index) => (&line[..index], &line[index..]),
        None => (line, ""),
    };
    let mut cursor = Cursor::new(rest);

    match keyword {
        "OBJ" => {
            single(&mut seen.objective, "OBJ")?;
            task.objective = cursor.string()?;
        }
        "SCOPE" => task.scope.push(cursor.string()?),
        "MUST" => push_requirement(task, &mut cursor, Strength::Hard, Polarity::Positive)?,
        "MUST_NOT" => push_requirement(task, &mut cursor, Strength::Hard, Polarity::Negative)?,
        "SHOULD" => push_requirement(task, &mut cursor, Strength::Soft, Polarity::Positive)?,
        "SHOULD_NOT" => push_requirement(task, &mut cursor, Strength::Soft, Polarity::Negative)?,
        "DO" => task.deliverables.push(cursor.string()?),
        "CTX" => {
            let reference = cursor.reference()?;
            if !is_valid_reference(&reference) {
                return Err(format!("invalid context reference {reference:?}"));
            }
            task.context.push(reference);
        }
        "DEP" => task.dependencies.push(cursor.string()?),
        "FACT" => task.facts.push(cursor.string()?),
        "ASSUME" => task.assumptions.push(cursor.string()?),
        "UNKNOWN" => task.unknowns.push(cursor.string()?),
        "ACCEPT" => task.acceptance.push(cursor.string()?),
        "EVIDENCE" => task.evidence.push(cursor.string()?),
        "PERM" => {
            single(&mut seen.permissions, "PERM")?;
            parse_permissions(task, &mut cursor)?;
        }
        "BUDGET" => {
            single(&mut seen.budget, "BUDGET")?;
            parse_budget(&mut task.budget, &mut cursor)?;
        }
        "SRC" => {
            single(&mut seen.source, "SRC")?;
            task.source = Some(cursor.string()?);
        }
        other => return Err(format!("unknown clause keyword {other:?}")),
    }

    cursor.finish()
}

fn single(seen: &mut bool, keyword: &str) -> Result<(), String> {
    if *seen {
        return Err(format!("{keyword} may appear only once"));
    }
    *seen = true;
    Ok(())
}

fn push_requirement(
    task: &mut Task,
    cursor: &mut Cursor<'_>,
    strength: Strength,
    polarity: Polarity,
) -> Result<(), String> {
    let text = cursor.string()?;
    task.requirements.push(Requirement {
        strength,
        polarity,
        text,
    });
    Ok(())
}

fn parse_permissions(task: &mut Task, cursor: &mut Cursor<'_>) -> Result<(), String> {
    let mut named: Vec<String> = Vec::new();
    while let Some(token) = cursor.token()? {
        let (name, value) = token
            .split_once('=')
            .ok_or_else(|| format!("expected name=value in PERM, found {token:?}"))?;
        if named.iter().any(|n| n == name) {
            return Err(format!("duplicate permission {name:?}"));
        }
        let value = match value {
            "permitted" => Permission::Permitted,
            "forbidden" => Permission::Forbidden,
            "unspecified" => Permission::Unspecified,
            other => return Err(format!("invalid permission value {other:?}")),
        };
        if !task.permissions.set(name, value) {
            return Err(format!("unknown permission {name:?}"));
        }
        named.push(name.to_string());
    }
    Ok(())
}

fn parse_budget(budget: &mut Budget, cursor: &mut Cursor<'_>) -> Result<(), String> {
    let mut named: Vec<String> = Vec::new();
    while let Some(token) = cursor.token()? {
        let (key, value) = token
            .split_once('=')
            .ok_or_else(|| format!("expected key=value in BUDGET, found {token:?}"))?;
        if named.iter().any(|n| n == key) {
            return Err(format!("duplicate budget key {key:?}"));
        }
        let number: u64 = value
            .parse()
            .map_err(|_| format!("budget {key} must be an unsigned integer, found {value:?}"))?;
        match key {
            "tokens" => budget.tokens = Some(number),
            "seconds" => budget.seconds = Some(number),
            other => return Err(format!("unknown budget key {other:?}")),
        }
        named.push(key.to_string());
    }
    Ok(())
}

fn requirement_keyword(requirement: &Requirement) -> &'static str {
    rosetta_core::label(requirement)
}

fn render_permissions(task: &Task) -> String {
    Permissions::NAMES
        .iter()
        .map(|name| {
            let value = match task.permissions.get(name).unwrap_or_default() {
                Permission::Permitted => "permitted",
                Permission::Forbidden => "forbidden",
                Permission::Unspecified => "unspecified",
            };
            format!("{name}={value}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_budget(budget: &Budget) -> String {
    let mut parts = Vec::new();
    if let Some(tokens) = budget.tokens {
        parts.push(format!("tokens={tokens}"));
    }
    if let Some(seconds) = budget.seconds {
        parts.push(format!("seconds={seconds}"));
    }
    parts.join(" ")
}

/// A byte cursor over the arguments of one clause.
struct Cursor<'a> {
    rest: &'a str,
}

impl<'a> Cursor<'a> {
    fn new(rest: &'a str) -> Self {
        Cursor { rest }
    }

    fn skip_whitespace(&mut self) {
        self.rest = self.rest.trim_start();
    }

    /// Parses one double-quoted string with escapes.
    fn string(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        let input: &'a str = self.rest;
        let mut chars = input.char_indices();
        match chars.next() {
            Some((_, '"')) => {}
            _ => return Err("expected a double-quoted string".to_string()),
        }
        let mut out = String::new();
        while let Some((index, ch)) = chars.next() {
            match ch {
                '"' => {
                    self.rest = &input[index + 1..];
                    return Ok(out);
                }
                '\\' => {
                    let (_, escape) = chars
                        .next()
                        .ok_or_else(|| "unterminated escape sequence".to_string())?;
                    match escape {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'u' => out.push(unicode_escape(&mut chars)?),
                        other => return Err(format!("invalid escape \\{other}")),
                    }
                }
                c if is_invisible_or_control(c) => {
                    return Err(format!(
                        "raw control or invisible character U+{:04X} in string; use an escape",
                        c as u32
                    ));
                }
                c => out.push(c),
            }
        }
        Err("unterminated string".to_string())
    }

    /// Parses a context reference, either bare or quoted.
    fn reference(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        if self.rest.starts_with('"') {
            return self.string();
        }
        self.token()?
            .ok_or_else(|| "expected a reference".to_string())
    }

    /// Returns the next whitespace-delimited token, or `None` at end of line.
    fn token(&mut self) -> Result<Option<String>, String> {
        self.skip_whitespace();
        if self.rest.is_empty() {
            return Ok(None);
        }
        if self.rest.starts_with('"') {
            return Err("unexpected quoted string".to_string());
        }
        let end = self
            .rest
            .find(char::is_whitespace)
            .unwrap_or(self.rest.len());
        let token = self.rest[..end].to_string();
        self.rest = &self.rest[end..];
        Ok(Some(token))
    }

    fn finish(&mut self) -> Result<(), String> {
        self.skip_whitespace();
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(format!("unexpected text {:?}", self.rest))
        }
    }
}

fn unicode_escape(chars: &mut std::str::CharIndices<'_>) -> Result<char, String> {
    match chars.next() {
        Some((_, '{')) => {}
        _ => return Err("expected \\u{HEX}".to_string()),
    }
    let mut digits = String::new();
    loop {
        match chars.next() {
            Some((_, '}')) => break,
            Some((_, c)) if c.is_ascii_hexdigit() && digits.len() < 6 => digits.push(c),
            _ => return Err("invalid \\u{HEX} escape".to_string()),
        }
    }
    let code =
        u32::from_str_radix(&digits, 16).map_err(|_| "invalid \\u{HEX} escape".to_string())?;
    char::from_u32(code).ok_or_else(|| format!("invalid code point U+{code:X}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# category: rust-refactoring
R1 split-parser
OBJ \"Split the parser module.\"
SCOPE \"crate:rosetta-encode\"
MUST_NOT \"Change the public API.\"
SHOULD \"Keep files small.\"
DO \"Move the tokenizer.\"
CTX file:src/lexer.rs
ACCEPT \"cargo test passes\"
PERM modify_files=permitted commit_changes=forbidden
BUDGET tokens=8000
SRC \"Please split the parser.\"
";

    #[test]
    fn parses_sample_document() {
        let task = parse(SAMPLE).unwrap();
        assert_eq!(task.id, "split-parser");
        assert_eq!(task.objective, "Split the parser module.");
        assert_eq!(task.requirements.len(), 2);
        assert_eq!(task.requirements[0].polarity, Polarity::Negative);
        assert_eq!(task.requirements[0].strength, Strength::Hard);
        assert_eq!(task.requirements[1].strength, Strength::Soft);
        assert_eq!(task.context, vec!["file:src/lexer.rs".to_string()]);
        assert_eq!(task.permissions.modify_files, Permission::Permitted);
        assert_eq!(task.permissions.run_commands, Permission::Unspecified);
        assert_eq!(task.budget.tokens, Some(8000));
        assert_eq!(task.budget.seconds, None);
        assert_eq!(task.source.as_deref(), Some("Please split the parser."));
    }

    #[test]
    fn round_trips_through_render() {
        let task = parse(SAMPLE).unwrap();
        let rendered = render(&task);
        assert_eq!(parse(&rendered).unwrap(), task);
    }

    #[test]
    fn round_trips_escapes_and_unicode() {
        let mut task = Task::new("escapes", "Quote \" and backslash \\ and tab\tend");
        task.deliverables.push("line one\nline two".into());
        task.acceptance.push("café ✓ 日本語".into());
        task.unknowns.push("bell \u{7} char".into());
        let parsed = parse(&render(&task)).unwrap();
        assert_eq!(parsed, task);
    }

    #[test]
    fn negative_requirements_survive_round_trip() {
        let mut task = Task::new("negatives", "Refactor");
        task.requirements.push(Requirement {
            strength: Strength::Hard,
            polarity: Polarity::Negative,
            text: "use unsafe code".into(),
        });
        let parsed = parse(&render(&task)).unwrap();
        assert_eq!(parsed.requirements, task.requirements);
    }

    #[test]
    fn soft_preference_keeps_its_strength() {
        let source = "R1 soft\nOBJ \"x\"\nSHOULD \"be brief\"\n";
        let task = parse(source).unwrap();
        assert_eq!(task.requirements[0].strength, Strength::Soft);
        assert!(render(&task).contains("SHOULD \"be brief\""));
    }

    #[test]
    fn unknown_clause_is_an_error_with_line_number() {
        let error = parse("R1 t\nOBJ \"x\"\nFROBNICATE \"y\"\n").unwrap_err();
        assert_eq!(error.line, 3);
        assert!(error.message.contains("unknown clause keyword"));
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let error = parse("R2 t\nOBJ \"x\"\n").unwrap_err();
        assert!(error.message.contains("unsupported schema version 2"));
    }

    #[test]
    fn missing_header_is_rejected() {
        let error = parse("OBJ \"x\"\n").unwrap_err();
        assert!(error.message.contains("expected header"));
    }

    #[test]
    fn invalid_task_id_is_rejected() {
        assert!(parse("R1 Bad_ID\n").is_err());
    }

    #[test]
    fn duplicate_objective_is_rejected() {
        let error = parse("R1 t\nOBJ \"a\"\nOBJ \"b\"\n").unwrap_err();
        assert!(error.message.contains("OBJ may appear only once"));
    }

    #[test]
    fn duplicate_permission_name_is_rejected() {
        assert!(parse("R1 t\nPERM network=permitted network=forbidden\n").is_err());
    }

    #[test]
    fn unknown_permission_is_rejected() {
        assert!(parse("R1 t\nPERM root=permitted\n").is_err());
    }

    #[test]
    fn invalid_permission_value_is_rejected() {
        assert!(parse("R1 t\nPERM network=maybe\n").is_err());
    }

    #[test]
    fn unterminated_string_is_rejected() {
        let error = parse("R1 t\nOBJ \"open\n").unwrap_err();
        assert!(error.message.contains("unterminated string"));
    }

    #[test]
    fn invalid_escape_is_rejected() {
        assert!(parse("R1 t\nOBJ \"bad \\q\"\n").is_err());
    }

    #[test]
    fn raw_control_character_is_rejected() {
        assert!(parse("R1 t\nOBJ \"bell \u{7}\"\n").is_err());
    }

    #[test]
    fn trailing_text_is_rejected() {
        assert!(parse("R1 t\nOBJ \"a\" \"b\"\n").is_err());
    }

    #[test]
    fn invalid_context_reference_is_rejected() {
        assert!(parse("R1 t\nCTX \"has space\"\n").is_err());
    }

    #[test]
    fn unicode_escape_is_decoded() {
        let task = parse("R1 t\nOBJ \"\\u{e9}\\u{1F600}\"\n").unwrap();
        assert_eq!(task.objective, "é😀");
    }

    #[test]
    fn bad_unicode_escape_is_rejected() {
        assert!(parse("R1 t\nOBJ \"\\u{110000}\"\n").is_err());
        assert!(parse("R1 t\nOBJ \"\\u0041\"\n").is_err());
    }

    #[test]
    fn budget_rejects_non_numeric_values() {
        assert!(parse("R1 t\nBUDGET tokens=lots\n").is_err());
    }

    #[test]
    fn split_documents_on_separator_lines() {
        let source = "R1 a\nOBJ \"x\"\n---\nR1 b\nOBJ \"y\"\n---\n\n";
        let documents = split_documents(source);
        assert_eq!(documents.len(), 2);
        assert_eq!(parse(documents[1]).unwrap().id, "b");
    }

    #[test]
    fn raw_invisible_and_bidi_characters_are_rejected() {
        // U+202E is a right-to-left override, U+200B a zero-width space.
        assert!(parse("R1 t\nOBJ \"safe\u{202e}text\"\n").is_err());
        assert!(parse("R1 t\nOBJ \"zero\u{200b}width\"\n").is_err());
    }

    #[test]
    fn escaped_invisible_characters_are_accepted_and_round_trip() {
        let task = parse("R1 t\nOBJ \"safe\\u{202e}text\"\n").unwrap();
        assert!(task.objective.contains('\u{202e}'));
        assert!(render(&task).contains("\\u{202e}"));
        assert_eq!(parse(&render(&task)).unwrap(), task);
    }

    #[test]
    fn quote_escapes_control_characters() {
        assert_eq!(quote("a\u{1}"), "\"a\\u{1}\"");
    }
}
