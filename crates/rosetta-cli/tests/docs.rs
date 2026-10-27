//! Checks that relative links in the project's Markdown files point to files
//! that exist. This runs as part of `cargo test`, so documentation drift fails
//! the build.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn markdown_files() -> Vec<PathBuf> {
    let root = repo_root();
    let mut files = vec![root.join("README.md"), root.join("todo.md")];
    let docs = root.join("docs");
    for entry in fs::read_dir(&docs).expect("docs directory exists") {
        let path = entry.expect("readable entry").path();
        if path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
    files
}

/// Extracts the targets of inline Markdown links, `[text](target)`.
fn link_targets(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = markdown;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else { break };
        out.push(after[..end].trim().to_string());
        rest = &after[end + 1..];
    }
    out
}

#[test]
fn relative_links_in_documentation_resolve() {
    let mut broken = Vec::new();
    for file in markdown_files() {
        let text = fs::read_to_string(&file).expect("markdown file readable");
        let base = file.parent().expect("file has a parent directory");
        for target in link_targets(&text) {
            if target.contains("://") || target.starts_with('#') || target.starts_with("mailto:") {
                continue;
            }
            let path_part = target.split('#').next().unwrap_or_default();
            if path_part.is_empty() {
                continue;
            }
            if !base.join(path_part).exists() {
                broken.push(format!("{}: {target}", file.display()));
            }
        }
    }
    assert!(broken.is_empty(), "broken links:\n{}", broken.join("\n"));
}

#[test]
fn link_extraction_finds_inline_links() {
    let targets = link_targets("See [a](docs/a.md) and [b](https://x.test/#y).");
    assert_eq!(targets, vec!["docs/a.md", "https://x.test/#y"]);
}
