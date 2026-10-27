# Security and permissions

Rosetta represents requests. It does not grant authority. Every rule below is
enforced by the code or pinned by a test, unless it says otherwise.

## Rules

1. **Encoding never broadens permissions.** Permissions come only from `PERM`
   clauses. Text in `OBJ`, `DO`, `ACCEPT` or elsewhere is data. The corpus
   contains injection-style tasks (`category: injection-adversarial`), and
   tests check that their stated permissions survive encoding.
2. **References are identifiers, not authorisation.** `CTX` entries name
   material. They never add a permission. See `integration.rs`.
3. **Prohibitions survive every transformation.** A `MUST_NOT` is kept through
   canonicalisation, all four encodings, and the DSL round trip. `diff` reports
   any dropped prohibition.
4. **Soft is never silently hard.** `SHOULD` renders as a preference in every
   encoding, and `MUST` as a requirement. Tests check both.
5. **Missing values are not defaults.** An unstated permission renders as
   `not stated` and raises warning `W002` or `W007`.
6. **Refusal falls back safely.** A refused task is sent as its original
   request, never as a rewritten encoding. A refused task with no original
   request is rejected, and nothing is produced in its place.
7. **Untrusted input is parsed strictly.** The parser rejects unknown clauses,
   bad escapes, raw control characters, duplicate singletons, and invalid
   identifiers, each with a line number. It never skips input silently.
8. **No secrets in the corpus or outputs.** Corpus tasks must not contain
   credentials or personal data. Trial records hold no prompt text.
9. **The host enforces real permissions.** Rosetta can say what a task asks
   for. Only the host running the tools can enforce it.

## Prompt framing

The English, concise and structured renderings each state that task text is
data and that the permissions listed are the only ones granted. This is a
framing aid, not a security boundary. A model may still follow injected
instructions, so the host must enforce permissions in code and must not rely
on the prompt.

## Out of scope

- Rosetta does not authenticate the requester or check who wrote a task.
- Rosetta does not sanitise prompts for secrets. Hosts should not place secrets
  in task text.
- Mixed-script look-alikes (for example a Cyrillic letter inside a Latin word)
  are flagged as `W010`. Words written entirely in one non-Latin script that
  imitate Latin words are not detected. See `architecture.md`.

## Reporting

Security issues in this experimental repository should be reported to the
maintainers privately before any public disclosure.
