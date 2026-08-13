# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring, read these

- **`CONTEXT.md`** at the repo root, or
- **`CONTEXT-MAP.md`** at the repo root if it exists.
  It points at one `CONTEXT.md` per context.
  Read each one relevant to the topic.
- **`docs/adr/`** contains decisions that may affect the work.
  In multi-context repositories, also check `src/<context>/docs/adr/` for context-scoped decisions.

If any of these files do not exist, **proceed silently**.
Do not flag their absence or suggest creating them upfront.
The `/domain-modeling` skill, reached through `/grill-with-docs` and `/improve-codebase-architecture`, creates them when terms or decisions are resolved.

## File structure

Single-context repo (most repos):

```
/
|-- CONTEXT.md
|-- docs/adr/
|   |-- 0001-event-sourced-orders.md
|   `-- 0002-postgres-for-write-model.md
`-- src/
```

## Use the glossary's vocabulary

When your output names a domain concept in a plan, branch, refactor proposal, hypothesis, or test name, use the term as defined in `CONTEXT.md`.
Do not drift to synonyms the glossary explicitly avoids.

If the concept you need is not in the glossary, reconsider whether you are inventing language the project does not use.
Record a real vocabulary gap for `/domain-modeling`.

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (event-sourced orders), but is worth reopening because..._
