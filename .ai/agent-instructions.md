# Agent Instructions

Portable operating instructions for any AI agent in this repository.
These rules are static and project-agnostic. Project-specific knowledge lives in `memory.md`.

## Read order (every session)
1. Read this file (`.ai/agent-instructions.md`).
2. Read `memory.md` if it exists (project summary + discoveries).
3. Read `local.md` if it exists (the current user's personal preferences).

## Precedence
When sources conflict, resolve in this order:
1. **Live code** always wins over any recorded memory (see Core behavior 3).
2. On a `local.md` vs `memory.md` conflict, ask once: "These conflict: <A> vs <B>. Which should I follow?" Record the answer in `local.md` so you never ask again.
3. In non-interactive contexts (no user to answer), prefer `local.md`, apply it, and note the choice in your output.

## Core behavior
1. **Don't hallucinate.** Never invent config options, APIs, files, or features. If it isn't in the code or in `memory.md`, verify by reading the code, or ask.
2. **Explore scoped to the task.** Read only files relevant to the current task. Use `memory.md` to skip rediscovery. Don't scan the whole codebase.
3. **Treat memory as a hint, not truth.** If a discovery contradicts the code you just read, trust the code and update the discovery.
4. **Be concise.** Minimize tokens: no redundant restating, no re-reading files already summarized in memory unless verifying.
5. **Log durable findings, not noise.** When you learn something reusable (architecture, build, gotchas), append it to `## Discoveries` in `memory.md` using the blueprint. One line. Skip the obvious.
6. **Every discovery needs a source.** A discovery without a `(Source: file/context)` is invalid — omit it rather than guess the source.
7. **Never write secrets.** Do not record API keys, tokens, passwords, PII, or machine-specific absolute paths in `memory.md` or `local.md`. If you encounter one, leave it out and warn the user.

## Keeping memory small (consolidate, don't reject)
- Never reject a finding just because the file is long; a compact summary still beats re-querying the whole codebase.
- Soft target: keep `memory.md` around ~150 lines. When it grows past that, **consolidate before appending**: merge related discoveries, compress the lowest-value lines, and drop only what is now redundant or superseded; never drop unique knowledge.

## Bootstrap (only if `memory.md` is missing)
If asked to explore/bootstrap, or if `memory.md` does not exist:
1. Create `memory.md` from the template below.
2. Do a *broad but shallow* pass: entry points, build/run config, languages, top-level module layout, test setup.
3. Fill `## Project` (3–5 sentences max) and seed `## Discoveries` with only high-value, durable facts.
4. Keep it small, since later agents read this every session. Prefer 10 sharp lines over 50 vague ones.

### memory.md template
```md
# Memory

## Project
<1 paragraph: what this project is, main languages, entry points, current state.>

## Discoveries
Blueprint: `- **[Category]** Short fact. Details. (Source: file/context)`
Categories: Architecture, Build, Config, Convention, Dependency, Gotcha, Pattern, Test, Tooling.
```

## Personal preferences
- Personal habits/conventions go in `local.md` (gitignored), never in `memory.md`.
- Only record preferences that interrupt flow if re-asked (formatting choice, preferred lib, naming, test framework).
- Before acting on a remembered preference, confirm briefly and proceed:
  "I remember you prefer X here, so I'll use X again. Proceed?"
- In non-interactive contexts, apply the remembered preference silently and note it in your output.