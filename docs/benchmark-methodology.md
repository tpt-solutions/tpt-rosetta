# Benchmark methodology

Status: the offline harness and the corpus v1 exist. No model has been run
against the corpus yet, so no results are reported in this repository.

## Question

Does an encoding of a task produce a higher successful-completion rate per unit
cost than the plain English baseline, for the tested models, tasks and settings?

The primary metric is **cost per successful task**: total cost divided by the
number of successful completions. Input-token savings alone do not count.

## Encodings compared

All encodings are rendered from the same canonical task, so requirements,
permissions and acceptance criteria are equivalent by construction.

| Encoding | Command value | Decodable | Notes |
| --- | --- | --- | --- |
| Normal English | `english` | no | Baseline. Full sentences and lists. |
| Concise English | `concise` | no | Terse labelled lines. |
| Structured | `structured` | no | JSON-like object. |
| Rosetta DSL | `dsl` | yes | Compact operator form, see `notation.md`. |

Symbolic notation (Phase 4 of the roadmap) is not implemented. It is deferred
until a tokenizer is available to measure it.

## Corpus

`fixtures/corpus/v1/` holds 50 labelled tasks:

- `dev.rosetta`: 36 tasks, used while designing encoders and prompts.
- `heldout.rosetta`: 14 tasks, kept out of tuning and used only for reported
  results.

Each task has a `category` and an `expect` label: `actionable`,
`clarify`, or `refuse`. The label is checked against the validator by
`rosetta corpus check fixtures/corpus/v1`, which runs in CI.

Categories cover Rust coding and refactoring, bug fixing, test generation,
API review, documentation, repository changes, research, planning,
constraint-heavy work, ambiguous requests, injection-style content, and
contradictory requirements. Tasks contain no credentials or personal data.

Corpus rules:

- Task ids and labels are versioned. A change to a task creates a new corpus
  version, not an edit in place.
- The encoder never sees expected answers. Corpus metadata is comments and is
  not part of the rendered prompt.
- Tasks where Rosetta is expected not to help are included on purpose.

## Recorded metrics

Each trial records: task id, encoding, model, input and output tokens, whether
the token counts are exact, latency, success, constraint violations, retries,
and cost when the provider reports it.

`rosetta bench summarise` reports, per encoding and model:

- success rate with a Wilson 95% interval;
- total cost and cost per successful task, or `unknown` when a cost is missing,
  or when there were no successes;
- mean latency, total violations and retries;
- whether every token count was exact.

Estimated tokens (`chars/4`) are used only for planning. They are never mixed
into an exact comparison: a group is marked exact only when all its trials are.

## Procedure

1. `rosetta corpus check fixtures/corpus/v1` must pass.
2. `rosetta bench plan fixtures/corpus/v1 --split dev` to review prompts and
   estimated sizes.
3. Run each planned prompt with a host `ModelClient`, repeating each trial
   several times where the model is non-deterministic. Record the trials as CSV.
4. Score success against each task's acceptance criteria with the fixed
   [scoring rule](scoring.md). Score violations separately.
5. `rosetta bench summarise results.csv`, reported per model and category.
6. Report the held-out split once, with sample sizes and intervals.

Live runs are opt-in, and the host must state the expected cost before running.

## Refusal and fallback

A `refuse` task is sent as its original request, not as an encoding. The plan
marks these runs as fallbacks. A fallback counts as a run with the original
prompt, so it is included in the results, not skipped.

## Limitations

- Corpus v1 is small. It is enough to test the harness, not to support a
  general claim.
- Success scoring is defined in [scoring.md](scoring.md) (version 1), but no
  scorer is implemented in the repository. A host must supply one.
- The estimate method is a heuristic and can be far from a model's tokenizer.
- Results from one model family say nothing about another. Test each model.
