# Recurring concepts in corpus v1

This note records which DSL clauses appear in the benchmark corpus
(`fixtures/corpus/v1`, 50 tasks: 36 dev, 14 held-out). It is a descriptive
count, taken with `grep` over the raw files. It does not propose a symbolic
vocabulary and does not measure token cost.

## Clause frequency

Counts are task-level occurrences of each clause keyword at the start of a line.

| Clause | Count | Concept |
|---|---|---|
| PERM | 50 | Permissions (every task has one) |
| OBJ | 50 | Objective |
| ACCEPT | 46 | Acceptance criteria |
| MUST | 37 | Hard constraint |
| DO | 37 | Deliverable or action |
| MUST_NOT | 36 | Prohibited action |
| SCOPE | 29 | Scope or file reference |
| SHOULD | 20 | Soft preference |
| UNKNOWN | 8 | Unknown or missing requirement |
| EVIDENCE | 4 | Evidence requirement |
| SRC | 3 | Original source text (provenance) |
| ASSUME | 1 | Assumption |

## Permission values

| Value | Count |
|---|---|
| commit_changes=forbidden | 50 |
| network=forbidden | 49 |
| modify_files=permitted | 41 |
| run_commands=permitted | 28 |
| run_commands=forbidden | 19 |
| modify_files=forbidden | 9 |
| run_commands=unspecified | 2 |
| network=permitted | 1 |

## Expected outcomes

| Expected outcome | Tasks |
|---|---|
| actionable | 30 |
| clarify | 13 |
| refuse | 7 |

## Reading the counts

- Objectives, acceptance criteria, and hard constraints or prohibitions
  account for most of the content. These are the concepts a notation must
  carry. Permissions appear in every task, so they are the strongest
  candidate for a compact, reusable form.
- Evidence and assumptions are rare. A vocabulary built only from this
  corpus would leave them under-tested.
- Most tasks forbid committing and network access. A default permission
  block could shorten the corpus, but only a model run can show whether
  omitting the explicit form hurts results. This is not tested here.
- The corpus is hand-written in one dialect. Counts describe this corpus
  only and do not show what real requests contain.

## Reproducing the counts

```sh
grep -c '^R1 ' fixtures/corpus/v1/*.rosetta
cat fixtures/corpus/v1/*.rosetta | grep -oE '^(OBJ|SCOPE|DO|MUST_NOT|MUST|SHOULD|ACCEPT|EVIDENCE|PERM|SRC|UNKNOWN|ASSUME)\b' | sort | uniq -c | sort -rn
```

`\b` does not match between `MUST` and `_`, so `MUST` and `MUST_NOT` are
counted separately.
