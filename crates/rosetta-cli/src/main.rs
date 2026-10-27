//! Command-line interface for TPT Rosetta.
//!
//! The CLI is offline. It validates, encodes, canonicalises and compares
//! tasks, and it plans and summarises benchmark runs. It makes no model calls.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rosetta_bench::metrics::parse_csv;
use rosetta_bench::{Split, load_dir, plan, summarise, summarise_by_category};
use rosetta_core::{Readiness, Task, canonicalise, diff, readiness, validate};
use rosetta_encode::dsl;
use rosetta_encode::{Encoding, Outcome, encode_or_fallback, round_trip_differences};

const USAGE: &str = "\
rosetta — TPT Rosetta (experimental). Offline commands only.

USAGE:
  rosetta validate <file>
      Parse and check each document. Exit 1 if any task is refused or invalid.
  rosetta encode <file> [--to english|concise|structured|dsl]
      Print the task in one encoding. Refused tasks print the original request.
  rosetta canon <file>
      Print the canonical DSL form of a task.
  rosetta diff <original> <reconstructed>
      List semantic differences. Exit 1 if they differ.
  rosetta roundtrip <file>
      Check that the DSL round trip preserves every field.
  rosetta corpus check <dir>
      Check that corpus labels match validator readiness.
  rosetta bench plan <dir> [--split dev|heldout] [--encodings a,b,...]
      List planned runs with estimated (not exact) input tokens.
  rosetta bench summarise <results.csv> [--by-category]
      Summarise recorded trials by encoding and model, optionally per category.
  rosetta help
";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let (command, rest) = match args.split_first() {
        Some((command, rest)) => (command.as_str(), rest),
        None => {
            print!("{USAGE}");
            return Ok(ExitCode::SUCCESS);
        }
    };
    match command {
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        "validate" => validate_command(rest),
        "encode" => encode_command(rest),
        "canon" => {
            let task = read_one(&required_path(rest.first(), "canon <file>")?)?;
            print!("{}", dsl::render(&canonicalise(&task)));
            Ok(ExitCode::SUCCESS)
        }
        "diff" => diff_command(rest),
        "roundtrip" => roundtrip_command(rest),
        "corpus" => corpus_command(rest),
        "bench" => bench_command(rest),
        other => Err(format!("unknown command {other:?}\n\n{USAGE}")),
    }
}

fn validate_command(args: &[String]) -> Result<ExitCode, String> {
    let path = required_path(args.first(), "validate <file>")?;
    let source = read_source(&path)?;
    let documents = dsl::split_documents(&source);
    let mut failed = false;

    for (index, document) in documents.iter().enumerate() {
        let label = format!("{}#{}", path.display(), index + 1);
        match dsl::parse(document) {
            Err(error) => {
                println!("{label}: parse error: {error}");
                failed = true;
            }
            Ok(task) => {
                let diagnostics = validate(&task);
                let verdict = readiness(&diagnostics);
                println!("{label}: {} ({verdict:?})", task.id);
                for diagnostic in &diagnostics {
                    println!("  {diagnostic}");
                }
                if verdict == Readiness::Refused {
                    failed = true;
                }
            }
        }
    }
    Ok(exit(failed))
}

fn encode_command(args: &[String]) -> Result<ExitCode, String> {
    let path = required_path(args.first(), "encode <file> [--to <encoding>]")?;
    let encoding = match flag_value(args, "--to")? {
        Some(name) => name.parse::<Encoding>()?,
        None => Encoding::English,
    };
    let task = read_one(&path)?;
    match encode_or_fallback(&task, encoding) {
        Outcome::Encoded {
            text, readiness, ..
        } => {
            print!("{text}");
            if readiness != Readiness::Actionable {
                eprintln!("note: readiness is {readiness:?}; clarify before sending");
            }
            Ok(ExitCode::SUCCESS)
        }
        Outcome::Fallback {
            original,
            diagnostics,
        } => {
            print!("{original}");
            for diagnostic in diagnostics {
                eprintln!("{diagnostic}");
            }
            eprintln!("note: task refused; printed the original request instead");
            Ok(ExitCode::SUCCESS)
        }
        Outcome::Rejected { diagnostics } => {
            for diagnostic in diagnostics {
                eprintln!("{diagnostic}");
            }
            Err("task refused and has no original request to fall back to".to_string())
        }
    }
}

fn diff_command(args: &[String]) -> Result<ExitCode, String> {
    let (Some(left), Some(right)) = (args.first(), args.get(1)) else {
        return Err("usage: rosetta diff <original> <reconstructed>".to_string());
    };
    let original = read_one(Path::new(left))?;
    let reconstructed = read_one(Path::new(right))?;
    let differences = diff(&original, &reconstructed);
    if differences.is_empty() {
        println!("no semantic differences");
        return Ok(ExitCode::SUCCESS);
    }
    for difference in &differences {
        println!("{difference}");
    }
    Ok(ExitCode::from(1))
}

fn roundtrip_command(args: &[String]) -> Result<ExitCode, String> {
    let task = read_one(&required_path(args.first(), "roundtrip <file>")?)?;
    let differences = round_trip_differences(&task, Encoding::Dsl)?;
    if differences.is_empty() {
        println!("round trip preserved every field");
        return Ok(ExitCode::SUCCESS);
    }
    for difference in &differences {
        println!("{difference}");
    }
    Ok(ExitCode::from(1))
}

fn corpus_command(args: &[String]) -> Result<ExitCode, String> {
    match args.first().map(String::as_str) {
        Some("check") => {
            let dir = required_path(args.get(1), "corpus check <dir>")?;
            let corpus = load_dir(&dir)?;
            let problems = rosetta_bench::corpus::check(&corpus);
            println!("{} tasks checked", corpus.len());
            for problem in &problems {
                println!("  {problem}");
            }
            Ok(exit(!problems.is_empty()))
        }
        _ => Err("usage: rosetta corpus check <dir>".to_string()),
    }
}

fn bench_command(args: &[String]) -> Result<ExitCode, String> {
    match args.first().map(String::as_str) {
        Some("plan") => bench_plan(&args[1..]),
        Some("summarise") => bench_summarise(&args[1..]),
        _ => Err("usage: rosetta bench plan <dir> | rosetta bench summarise <csv>".to_string()),
    }
}

fn bench_plan(args: &[String]) -> Result<ExitCode, String> {
    let dir = required_path(args.first(), "bench plan <dir>")?;
    let split = match flag_value(args, "--split")? {
        None => None,
        Some("dev") => Some(Split::Dev),
        Some("heldout") => Some(Split::Heldout),
        Some(other) => return Err(format!("unknown split {other:?}")),
    };
    let encodings: Vec<Encoding> = match flag_value(args, "--encodings")? {
        None => Encoding::ALL.to_vec(),
        Some(list) => list
            .split(',')
            .map(|name| name.trim().parse::<Encoding>())
            .collect::<Result<_, _>>()?,
    };

    let corpus: Vec<_> = load_dir(&dir)?
        .into_iter()
        .filter(|item| split.is_none_or(|s| item.split == s))
        .collect();
    let runs = plan(&corpus, &encodings);

    println!("task\tsplit\tencoding\tfallback\testimated_input_tokens");
    for run in &runs {
        println!(
            "{}\t{:?}\t{}\t{}\t{}",
            run.task_id, run.split, run.encoding, run.fallback, run.estimated_input.tokens
        );
    }
    let total: u64 = runs.iter().map(|r| r.estimated_input.tokens).sum();
    eprintln!(
        "{} planned runs, {} fallbacks, estimated input tokens {} (estimates only; no model calls made)",
        runs.len(),
        rosetta_bench::plan::fallback_count(&runs),
        total
    );
    Ok(ExitCode::SUCCESS)
}

fn bench_summarise(args: &[String]) -> Result<ExitCode, String> {
    let path = required_path(args.first(), "bench summarise <csv>")?;
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let trials = parse_csv(&text)?;
    let summaries = if has_flag(args, "--by-category") {
        summarise_by_category(&trials)
    } else {
        summarise(&trials)
    };
    println!(
        "encoding\tmodel\tcategory\ttrials\tsuccesses\tsuccess_rate\twilson_95\ttotal_cost_usd\tcost_per_success_usd\tmean_latency_ms\tviolations\tretries\texact_tokens"
    );
    for s in summaries {
        let cost = |value: Option<f64>| value.map_or("unknown".to_string(), |v| format!("{v:.6}"));
        println!(
            "{}\t{}\t{}\t{}\t{}\t{:.3}\t[{:.3}, {:.3}]\t{}\t{}\t{:.1}\t{}\t{}\t{}",
            s.encoding,
            s.model,
            s.category,
            s.trials,
            s.successes,
            s.success_rate,
            s.wilson_low,
            s.wilson_high,
            cost(s.total_cost_usd),
            cost(s.cost_per_success_usd),
            s.mean_latency_ms,
            s.total_violations,
            s.total_retries,
            s.tokens_exact
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// Reads a file that must contain exactly one document.
fn read_one(path: &Path) -> Result<Task, String> {
    let source = read_source(path)?;
    let documents = dsl::split_documents(&source);
    match documents.as_slice() {
        [document] => dsl::parse(document).map_err(|e| format!("{}: {e}", path.display())),
        [] => Err(format!("{} contains no document", path.display())),
        _ => Err(format!(
            "{} contains {} documents; expected one",
            path.display(),
            documents.len()
        )),
    }
}

fn read_source(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

fn required_path(value: Option<&String>, usage: &str) -> Result<PathBuf, String> {
    value
        .map(PathBuf::from)
        .ok_or_else(|| format!("usage: rosetta {usage}"))
}

/// Returns the value after `flag`, or `None` if the flag is absent.
fn flag_value<'a>(args: &'a [String], flag: &str) -> Result<Option<&'a str>, String> {
    match args.iter().position(|arg| arg == flag) {
        None => Ok(None),
        Some(index) => args
            .get(index + 1)
            .map(|value| Some(value.as_str()))
            .ok_or_else(|| format!("{flag} needs a value")),
    }
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn exit(failed: bool) -> ExitCode {
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_value_reads_following_argument() {
        let args: Vec<String> = ["file.rosetta", "--to", "dsl"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(flag_value(&args, "--to").unwrap(), Some("dsl"));
        assert_eq!(flag_value(&args, "--split").unwrap(), None);
    }

    #[test]
    fn flag_without_value_is_an_error() {
        let args = vec!["--to".to_string()];
        assert!(flag_value(&args, "--to").is_err());
    }
}
