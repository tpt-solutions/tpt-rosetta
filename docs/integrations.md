# Integrations

Status: design review and a prototype only. None of these integrations is
wired into another repository. Every point below was checked against the local
checkouts on the maintainer's machine, not against a published interface.

## TPT Weave

Reviewed: `tpt-weave` (pre-alpha; Phases 0–17 implemented). Its context
retrieval returns symbol records and rendered text from its own index
(`tpt-weave-context`, `Retriever`). It delivers content, and it has its own
cache and token accounting.

Design consequences:

- Weave should supply **identifiers** for the context it selects, such as file
  paths or symbol keys. Rosetta stores them as `CTX` references and never
  copies content. This keeps Rosetta out of retrieval and indexing, as the spec
  requires.
- Availability must be checked. A reference Weave cannot resolve must be
  reported to the model, not dropped. `ContextResolver` and
  `attach_context_references` in `crates/rosetta-encode/src/integration.rs`
  implement that check for a host.
- Weave's own caching and token reduction already save tokens. Rosetta's
  benefit has to be measured on top of them. Measuring Weave's savings
  separately avoids counting them twice.

Prototype: `attach_context_references` adds validated, de-duplicated references
and returns the unavailable ones. Tests show that permissions and constraints
are unchanged.

## TPT Code Command Center

Reviewed: `tpt-code-command-center` (a TypeScript VS Code extension, cloned
locally). Files cited are in `src/proxy/pipeline.ts`, `src/proxy/server.ts` and
`src/ledger/ledger.ts`.

How it works today:

- A local proxy runs a fixed sequence of stages over the whole request body:
  Vault, Token Shield, Memory Weaver, Smart Context (with Weave skeletons),
  Silent Edit, Prompt Cache, Mistral cache key, then Router.
- Each stage rewrites the JSON body in place. The stages are synchronous or
  awaited in order, and each one appends a label to `moduleActions`.
- Savings are recorded in a ledger table with tokens in and out, cost, a cache
  hit flag and the comma-joined module actions.

Findings that shape a Rosetta stage:

1. **A stage error breaks the request.** `server.ts` wraps `runPipeline` in a
   try/catch that returns HTTP 500 (`TPT pipeline error`). It does not fall
   back to the original body. A Rosetta stage must therefore catch its own
   errors and return the unchanged body, which is the fallback contract.
2. **Only the task should be rewritten.** The body holds the whole chat
   history, the system prompt and tool results. Rosetta can only encode the
   latest user request, so it needs task extraction first, and it must not
   touch the system prompt. Silent Edit injects into the system prompt, so the
   Rosetta stage should not run on it.
3. **Placement.** The stage belongs after Smart Context and before Silent Edit
   and Prompt Cache. Prompt Cache marks stable prefixes, and rewriting the
   latest turn leaves those prefixes alone. It must run after Vault, so secrets
   are redacted before Rosetta sees the text.
4. **Overlap with existing savings.** Smart Context and Weave already reduce
   file content inline, and Token Shield and Prompt Cache already reduce cost.
   Rosetta's gain must be measured on top of these, and the ledger must show it
   separately.
5. **Measurement gap.** The ledger does not record per-stage token counts. It
   stores module actions only as text. Before a Rosetta stage is compared, the
   pipeline should record tokens before and after each stage, so savings can
   be attributed without parsing strings. Savings are also estimated from
   average hit cost, not measured per request.

Rosetta must remain an optional stage, disabled by default. The fallback path
(`Outcome::Fallback`) is the contract it has to honour, and the stage must
never turn an error into a 500.

## TPT-UIR

Not checked in code. Proposed boundary, from the spec: a Rosetta task may
reference a UIR artifact as an identifier, for example `CTX uir:<artifact-id>`,
but UIR gains no task-intent semantics. The identifier syntax already allows a
colon and a slash, so no schema change is needed.

## TPT Telos

Not checked in code. Proposed boundary: formal checks only for constraints with
a precise mapping and supported proof semantics. A Rosetta validator result is
structural or heuristic. It must never be labelled a formal proof.

## Summary of what is and is not done

| Item | Status |
| --- | --- |
| Weave review | Done for the public retrieval surface. |
| Weave integration prototype | Done as a standalone function. Not wired to Weave. |
| Command Center review | Done for the proxy pipeline, error path and ledger. Stage not implemented. |
| UIR and Telos boundaries | Proposed in writing. Not checked in code. |
