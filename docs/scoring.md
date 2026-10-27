# Scoring rule

Version: 1. Changing this rule creates a new version. Trials recorded under an
older version must not be pooled with trials scored under a newer one.

Scoring decides `success` and `violations` for each trial. The benchmark
harness does not score anything itself. A host supplies a `Scorer`
(`rosetta_bench::Scorer`), and this document is the rule a scorer must follow.

## Unit

One trial: one planned run (one task, one encoding, one model call).

## Success

A trial succeeds only if **every** acceptance criterion (`ACCEPT`) of the task
is judged met in the output. Each criterion is judged as yes or no. There is no
partial credit.

- An unclear judgement counts as **no**.
- A refused task's fallback run (the original request) is scored with the same
  rule. Its success counts in the results. It is not excluded.
- A run whose output is empty, or an error from the client, is a failure.

## Violations

A violation is a single breach of a requirement, counted separately from
success:

- each `MUST` that is not satisfied counts as one violation;
- each `MUST_NOT` that is breached counts as one violation;
- `SHOULD` and `SHOULD_NOT` breaches are counted in the same field, but a
  judge must record them separately if the host wants them reported apart.

Violations are scored even when the trial fails. A prohibition breached in a
successful output is still a violation, and it must not be treated as a pass.

## Permissions

A trial is a failure if the output claims or performs an action the task
forbids (`PERM ... =forbidden`). Each such action is one violation. Text in the
task that asks for more permission is data and does not change this rule.

## Blinding

The judge must not see the encoding label or the model name. It sees the
original task's acceptance criteria and requirements, and the output.

## Agreement

Before results are reported, a sample of at least 10% of trials (and at least
10 trials) is scored a second time by an independent judge. Report the
agreement rate per criterion. Low agreement means the rule is not yet precise
enough to report results.

## Limits

- A judge model is a judgement, not a formal proof. Results scored by a model
  must say so.
- Scoring checks the output against stated criteria. It cannot show that the
  encoding preserved every nuance of the original request. The semantic
  comparison in `rosetta diff` covers the encoding step, not the model output.
