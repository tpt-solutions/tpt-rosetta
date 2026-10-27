TPT Rosetta --- TODO
Status: Initial roadmap. Offline foundation built (Phases 0–3, 5, 7–8 in part).
Evaluation phases need live model runs and are not started. Priority: Evidence
first; keep the initial scope small.

Legend: [x] done in this repository; [ ] not done. Notes after a task say what
is missing.

Phase 0 --- Repository foundation
- [ ] Create tpt-rosetta repository under tpt-solutions.
  Note: publishing, deliberately excluded. The repo is not a git repository yet.
- [x] Add SPEC.txt, TODO.md, README.md, and MIT license.
  Note: files are spec.txt and todo.md (lowercase), and the copyright holder
  "TPT Solutions" is an assumption to confirm.
- [x] State clearly that Rosetta is experimental and makes no
  universal token-saving claims.
- [x] Record scope boundaries against tpt-weave,
  tpt-code-command-center, tpt-uir, and TPT Script.
- [x] Establish Rust edition, MSRV, formatting, linting, test, and CI
  policies consistent with the TPT ecosystem.
  Note: edition 2024, MSRV 1.85 (verified: tests pass on 1.85.1), rustfmt.toml,
  clippy -D warnings, and .github/workflows/ci.yml. CI has not been run remotely.
- [x] Verify dependency licenses and avoid unnecessary dependencies.
  Note: one dependency, unicode-normalization (MIT or Apache-2.0, with
  Unicode-3.0 data). Its transitive dependency is Zlib, Apache-2.0 or MIT. The
  reason is recorded in docs/architecture.md.

Phase 1 --- Baseline and benchmark design
- [x] Define benchmark methodology before implementing symbolic
  compression. (docs/benchmark-methodology.md)
- [x] Select an initial set of 50--100 representative tasks.
  Note: 50 tasks (36 dev, 14 held-out) in fixtures/corpus/v1.
- [x] Include Rust coding, refactoring, debugging, testing,
  documentation, research, planning, and constraint-heavy tasks.
- [x] Give each task explicit acceptance criteria and a scoring
  method.
  Note: acceptance criteria are in every task. The scoring rule is version 1
  in docs/scoring.md. A host supplies the scorer (rosetta_bench::Scorer).
- [x] Create a versioned task corpus and fixture format.
- [x] Define equivalent-context rules for comparing encodings.
  Note: all encodings render from one canonical task.
- [x] Implement an English baseline runner.
  Note: rosetta_bench::run_plan executes planned runs through a host
  ModelClient and records trials. It is tested with a fake client. No live run
  has been made.
- [x] Implement a concise-English baseline. (rosetta-encode, `concise`)
- [x] Add a structured-data baseline, such as JSON or a compact
  key/value format. (rosetta-encode, `structured`)
- [x] Record model, tokenizer, settings, task version, input/output
  tokens, latency, cost, success, retries, and constraint violations
  where available. (Trial record and CSV format in rosetta-bench)
- [x] Mark estimated token counts separately from exact tokenizer
  counts. (`tokens_exact` flag; estimates are labelled)
- [x] Keep API-based runs opt-in and disclose expected costs.
  Note: the policy is documented and enforced by design (no provider client
  in the repo). Cost disclosure is not automated.
- [x] Define a held-out task set to reduce benchmark overfitting.

Phase 2 --- Canonical semantic task model
- [x] Define a minimal typed representation for objectives, scope,
  constraints, permissions, context references, acceptance criteria,
  and evidence requirements. (rosetta-core)
- [x] Distinguish hard constraints from preferences.
- [x] Distinguish facts from assumptions and unknowns.
- [x] Represent prohibited actions explicitly. (MUST_NOT)
- [x] Represent missing or ambiguous requirements without silently
  inventing values. (Unspecified permissions, W001, W002, W007, W004)
- [x] Define stable identifiers and schema versioning.
- [x] Add deterministic canonicalisation.
- [x] Add human-readable diagnostics.
- [x] Add validation for conflicting or incomplete fields.
- [x] Add examples and counterexamples to the specification.
  Note: examples/ and the refuse and clarify tasks in the corpus.
- [x] Keep the schema small until benchmark evidence supports
  expansion.

Phase 3 --- Initial encoders and parser
- [x] Implement serialization for the canonical model.
- [x] Implement a readable structured-text encoding.
- [x] Implement a compact DSL candidate. (DSL 1)
- [x] Create a notation reference documenting every operator and
  field. (docs/notation.md)
- [x] Define parsing errors and unknown-symbol behavior.
- [x] Add encode/decode round-trip tests where supported.
- [x] Test preservation of negative constraints and permissions.
- [x] Test Unicode normalization, escaping, and malformed input.
  Note: canonical text is NFC (unicode-normalization). Non-NFC task text warns
  (W009). Raw invisible and bidi characters are rejected in source and flagged
  as W008 in tasks. Words mixing Latin with Greek, Cyrillic or Armenian letters
  warn (W010). Words written entirely in one look-alike script are not
  detected. See docs/architecture.md.
- [x] Ensure an encoding failure can fall back to the original
  request. (encode_or_fallback)
- [x] Do not assume visual brevity implies tokenizer efficiency.
  Note: no efficiency claim is made anywhere in the repository.

Phase 4 --- Symbolic notation experiments
Note: deferred. Measuring symbol cost needs a tokenizer or a model run, and
neither is available here. The DSL uses ASCII keywords only.
- [ ] Identify recurring semantic concepts from the benchmark corpus.
- [ ] Propose a small, versioned vocabulary for objectives,
  constraints, dependencies, evidence, uncertainty, and verification.
  Note: DSL 1 keywords cover these concepts, but this is not a symbolic
  vocabulary and has not been compared with alternatives.
- [x] Give every symbol a precise definition, type or arity where
  applicable, and examples. (docs/notation.md)
- [ ] Compare ASCII operators, familiar mathematical notation, Unicode
  symbols, and readable identifiers.
- [ ] Measure the token cost of both the notation and any legend or
  definition required by the model.
- [ ] Test whether a reusable notation definition can be cached or
  supplied without compromising semantics.
- [ ] Test models independently rather than assuming a universal
  notation.
- [ ] Remove symbols that increase errors or fail to improve
  end-to-end results.

Phase 5 --- Semantic preservation and safety
- [x] Preserve original user text and transformation provenance for
  evaluation. (SRC clause, kept exactly by canonicalisation)
- [x] Add diagnostics for ambiguous natural-language requests.
- [x] Prevent soft preferences from silently becoming hard
  requirements. (tested in rosetta-encode)
- [x] Prevent prohibited actions from disappearing during encoding.
  (tested; `diff` reports dropped prohibitions)
- [x] Compare reconstructed tasks against labelled source
  requirements. (`rosetta diff`, structural comparison)
- [x] Test adversarial inputs and prompt-injection-like content.
  (injection-adversarial corpus tasks; parser tests)
- [x] Ensure references do not confer tool permissions.
- [x] Document the difference between structural validation and
  semantic correctness. (docs/architecture.md, docs/security.md)
- [x] Define conditions under which Rosetta must refuse conversion or
  request clarification. (docs/security.md, readiness rules)

Phase 6 --- Evaluation and decision gate
Note: all of these need live model runs. None has been run, so no results
exist in this repository.
- [ ] Run all baseline encodings on the same task set.
- [ ] Repeat variable model trials where practical.
- [ ] Report results by model and task category.
  Note: summarise_by_category and `bench summarise --by-category` exist. No
  results have been recorded yet.
- [x] Calculate total cost per successful task, not only input-token
  savings. (rosetta-bench metrics, `bench summarise`)
- [ ] Compare acceptance-test pass rate, retries, latency, and
  constraint violations.
  Note: the summary reports these fields, but there are no results yet.
- [ ] Include negative and inconclusive findings in the report.
- [ ] Set a quality threshold for any claimed efficiency improvement.
- [ ] Decide whether symbolic encoding offers sufficient benefit to
  continue.
- [ ] Do not begin custom tokenizer training unless results justify
  it.

Phase 7 --- Rust library and CLI
- [x] Finalize crate boundaries based on implemented functionality.
  (four crates; see docs/architecture.md)
- [x] Expose a stable API for parsing, validation, canonicalisation,
  and encoding.
  Note: the API is public at version 0.1.0 and may still change.
- [x] Add a CLI for converting, validating, comparing, and
  benchmarking representations. (`rosetta validate|encode|canon|diff|roundtrip|corpus|bench`)
- [x] Provide deterministic offline tests. (no network access needed)
- [x] Add examples and user documentation. (examples/, README, docs/)
- [x] Keep provider credentials and network access out of the core
  library.
- [x] Add version-compatibility tests and migration notes.
  Note: unsupported versions are rejected and tested. No migration is
  needed until a second schema exists.

Phase 8 --- TPT integrations
- [x] Review the current implementation of tpt-weave before
  designing its integration. (docs/integrations.md)
- [x] Prototype supplying Weave-selected context through explicit
  references. (attach_context_references; not wired to Weave)
- [x] Ensure referenced context is actually available to the model or
  tool when required. (ContextResolver reports unavailable references; the
  host implements the check)
- [x] Review tpt-code-command-center before adding any
  transformation stage.
  Note: reviewed the proxy pipeline, error path and ledger. Findings are in
  docs/integrations.md. The stage itself is not implemented, and it must stay
  optional.
- [x] Make Rosetta optional and preserve a safe fallback to the
  original prompt. (Outcome::Fallback)
- [ ] Benchmark incremental benefits on top of existing context
  reduction, caching, and summarisation.
  Note: needs live runs.
- [ ] Avoid double-counting savings from existing components.
  Note: the rule is written in docs/integrations.md. Nothing is measured yet.
- [x] Define how a Rosetta task may reference a computational UIR
  artifact without changing UIR's core purpose. (proposed in
  docs/integrations.md; not checked in code)
- [x] Explore formal checks with tpt-telos only for properties with
  supported semantics and proof methods. (boundary documented; not checked
  in code)

Phase 9 --- Release readiness
- [x] Complete security, limitations, architecture, and benchmark
  documentation. (docs/)
- [x] Ensure all public claims match measured results.
  Note: the repository makes no measured efficiency claims.
- [x] Publish reproducible benchmark instructions and result formats.
  (docs/benchmark-methodology.md; CSV format in rosetta-bench)
- [x] Add CI checks for formatting, linting, tests, and documentation.
  Note: .github/workflows/ci.yml covers format, clippy, tests, corpus check,
  and MSRV. Relative links in the Markdown files are checked by a test in
  rosetta-cli, which runs under cargo test. CI has not been run remotely.
- [x] Verify all dependencies and licenses.
- [ ] Tag an initial research release only when acceptance criteria in
  SPEC.txt are met.
  Note: publishing, excluded. Not met yet: outcomes are not scored and no
  model has been run (spec §17).
- [ ] Publish model/task-specific results and known regressions.
  Note: no results exist yet.
- [ ] Decide whether the project merits broader adoption, continued
  research, or integration-only maintenance.

Explicitly deferred
- [ ] Custom tokenizer or vocabulary training.
- [ ] Training a new LLM to reason natively in Rosetta.
- [ ] A universal replacement for English.
- [ ] An exhaustive ontology of all human concepts.
- [ ] Mandatory Rosetta processing for every Command Center request.
- [ ] Duplicating repository indexing, context retrieval, model
  routing, or workflow orchestration.
