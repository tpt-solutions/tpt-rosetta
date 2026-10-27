# Architecture

Status: experimental. Schema version 1. Crates are at version 0.1.0 and are not
published.

## Crates

| Crate | Depends on | Responsibility |
| --- | --- | --- |
| `rosetta-core` | nothing | Canonical task model, validation, readiness, canonicalisation, diff. |
| `rosetta-encode` | `rosetta-core` | DSL parser and renderer, four surface encodings, fallback, host context references. |
| `rosetta-bench` | `rosetta-core`, `rosetta-encode` | Corpus loading and checks, run planning, token estimates, trial metrics. |
| `rosetta-cli` | all three | The `rosetta` binary. Offline commands only. |

The only third-party dependency is `unicode-normalization` (in `rosetta-core`),
which provides NFC normalisation. Its licence is MIT or Apache-2.0, with
Unicode data under Unicode-3.0. Its transitive dependency `tinyvec` is
Zlib, Apache-2.0 or MIT. Its MSRV is 1.36, well below the workspace's 1.85. The
reason for adding it is recorded under "Unicode" below. Any further dependency
needs the same kind of recorded reason.

## Data flow

```text
DSL text ──parse──▶ Task ──validate──▶ diagnostics ──readiness──▶ verdict
                     │                                   │
                     ├──canonicalise──▶ Task (canonical) │
                     │                                   ▼
                     │                     Actionable | NeedsClarification | Refused
                     ▼                                   │
              render(encoding) ◀─────── encode_or_fallback ┘
                     │                    Refused → original request (SRC)
                     ▼
               prompt text ──(host, opt-in)──▶ model ──▶ trial ──▶ metrics
```

## Principles applied in code

- **Missing is not default.** An unstated permission is `Unspecified`. It is
  never rendered as forbidden or permitted, and it produces a warning.
- **Strength and polarity are never changed.** `SHOULD` stays soft and
  `MUST_NOT` stays a prohibition through canonicalisation, every encoding, and
  the DSL round trip. Tests in `rosetta-encode` pin this.
- **Failing safe.** A refused task is never sent as a rewritten encoding. The
  original request is used instead, or nothing is sent if no original exists.
- **Offline by default.** Nothing in the workspace makes a network call. A
  live model call requires a host `ModelClient` implementation, which lives
  outside this repository.
- **Estimates are labelled.** Token counts from `rosetta-bench` carry
  `exact: false` and their method. No tokenizer is bundled.

## Unicode

- **Canonical form is NFC.** `canonicalise()` composes every task text field
  to NFC, then collapses whitespace. Decomposed and precomposed input of the
  same text therefore produce the same canonical task.
- **`source` is kept exactly.** It is the audit record, so it is never
  normalised.
- **Non-NFC input warns.** Validation emits `W009` when task text is not NFC,
  so the author sees that canonicalisation changes the bytes.
- **Mixed-script words warn.** Validation emits `W010` for any word in task
  text that mixes Latin letters with Greek, Cyrillic or Armenian letters. This
  is the usual spoofing pattern, for example a Cyrillic "а" inside "paypal". It
  is detected from Unicode block ranges, so it needs no table. The `source`
  field is not scanned, because it is kept as the audit record.
- **Invisible and bidi controls are refused in source and flagged in tasks.**
  The parser rejects raw zero-width and bidi-override characters and requires
  an escape. The validator emits `W008` for any task text that contains them.

## Canonical model

See `crates/rosetta-core/src/lib.rs`. Fields are explicit and typed. The
schema is kept small on purpose, as the spec asks: a field is added only when
benchmark evidence needs it.

## Versioning

`SCHEMA_VERSION` is `1`. The DSL header `R1` must match it. A different version
is rejected by the parser and by the validator. Migrations will be written
here when a second version exists. None is needed yet.

## Known gaps

- **Pure-script look-alikes are not detected.** A word written entirely in
  Cyrillic or Greek that looks like a Latin word (for example "аррӏе") passes.
  Detecting it needs the full UTS #39 confusables table, which is not included.
  Words that mix Latin with those scripts are detected (W010), and so are
  invisible and bidi controls (W008).
- **No semantic checker.** The validator checks structure and readiness. It
  cannot tell whether an encoding preserved the meaning of free text. That
  needs labelled outcomes from the benchmark.
- **No live benchmark.** The harness plans and summarises. Recording real
  trials needs a host model client, an opt-in decision, and a cost estimate.
