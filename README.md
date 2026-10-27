# TPT Rosetta

**Status: experimental research project.** Rosetta has no released API, no
stable schema, and no measured results yet. Nothing in this repository should
be read as a claim that it saves tokens, cost, or time.

TPT Rosetta investigates whether AI task requests can be represented more
compactly and precisely than ordinary natural-language prompts without
reducing task success, correctness, or important nuance.

The project will provide:

- a canonical, model-independent semantic representation of task intent,
  constraints, permissions, context references, and acceptance criteria;
- encoders that render that representation as ordinary English, concise
  English, structured text, and experimental symbolic notation;
- a benchmark harness that scores task outcomes, constraint violations, and
  cost per successful task rather than token length alone.

The governing documents are:

- [spec.txt](spec.txt) — the initial design specification (v0.1.0 proposal).
- [todo.md](todo.md) — the phased roadmap and current task list.

## Principles

- Measure, do not assume. Shorter strings, invented symbols, and fewer
  apparent words are not evidence of fewer model tokens or better results.
- Negative and inconclusive findings are reported, not hidden.
- Requests are representations, not grants of authority. Encoding a request
  must never broaden its permissions, and references to tools or resources are
  identifiers, not authorisation.
- Treat prompts, retrieved documents, repository content, and model output as
  untrusted data.
- Rosetta is usable as a standalone library and CLI. Integrations with other
  TPT repositories are optional and must preserve a safe fallback to the
  original prompt.

## Scope boundaries

Rosetta owns the semantic task schema, notation experiments, encoders,
decoders, validators, and representation-quality benchmarks.

Rosetta does not own:

- **TPT Weave** — repository indexing, context retrieval, and context
  selection.
- **TPT Code Command Center** — request interception, provider and model
  routing, credentials, and workflow orchestration.
- **TPT-UIR** — the typed computational intermediate representation. Rosetta
  is not a generic computation language.
- **TPT Script** — general-purpose scripting.
- **TPT Telos** — formal verification. Rosetta may reference it for properties
  with a precise mapping and supported proof semantics. An ordinary LLM
  judgment is never labelled a formal proof.

The integration boundaries above are proposals. Check the implementation of
any adjacent repository before relying on a feature from it.

## Non-goals

- Replacing human languages in general communication.
- Requiring a custom tokenizer, vocabulary, or trained model.
- Guaranteeing fewer tokens for every model or task.
- Executing instructions or modifying repositories on its own.

## Claims

No universal token-saving or efficiency claim is made. Any efficiency claim
will be limited to the models, tasks, settings, and evaluation conditions that
were actually tested, and will report regressions.

## Status of this repository

The Rust workspace builds and its offline tests pass. It has four crates:

- `crates/rosetta-core`: canonical task model, validation, canonicalisation, diff.
- `crates/rosetta-encode`: DSL parser and renderer, four encodings, fallback.
- `crates/rosetta-bench`: corpus loader, run planning, token estimates, metrics.
- `crates/rosetta-cli`: the `rosetta` command.

Try it with the example task:

```sh
cargo run -q -p rosetta-cli -- validate examples/split-parser.rosetta
cargo run -q -p rosetta-cli -- encode examples/split-parser.rosetta --to concise
cargo run -q -p rosetta-cli -- corpus check fixtures/corpus/v1
```

Documentation lives in [docs/](docs/): [notation](docs/notation.md),
[architecture](docs/architecture.md), [benchmark methodology](docs/benchmark-methodology.md),
[security](docs/security.md) and [integrations](docs/integrations.md).

No model has been run against the benchmark corpus, so the repository has no
measured results. See [todo.md](todo.md) for what remains.

## Licence

Licensed under either of the MIT Licence ([LICENSE-MIT](LICENSE-MIT)) or the Apache Licence, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)), at your option.
