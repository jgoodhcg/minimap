---
version: "2026-09-14"
---

# Agent Blueprint

Reference for consistent agent behavior across projects. Copy into any project and reference from `AGENTS.md`. Do not edit a project's copy; propose changes in the `agent-blueprint` repo and re-sync.

---

## Core Invariants

Use these IDs in alignment reports for deterministic, machine-checkable outcomes. A **MUST** rule fails an alignment report when unmet. A **SHOULD** rule is a recommended default; a project can decline it and record the reason in `AGENTS.md`.

**MUST**
- `BP-CORE-01` `AGENTS.md` exists and references `AGENT_BLUEPRINT.md`.
- `BP-CORE-02` `roadmap/index.md` exists.
- `BP-CORE-03` Work in progress lives in `roadmap/` work unit files with valid frontmatter.
- `BP-CORE-04` Agents execute `ready` work units autonomously and self-validate before returning.
- `BP-CORE-05` Commits happen only after explicit user approval.
- `BP-CORE-06` Alignment responses use the required report format in this blueprint.
- `BP-CORE-09` `AGENTS.md` stores a commit trailer template (placeholders), not concrete co-author/provider/model values.
- `BP-CORE-11` On conflicting instructions, apply the precedence order in `[BP-PRECEDENCE]`.
- `BP-CORE-12` Completion reports name the checks that ran and the checks that did not (`[BP-VERIFY]`).
- `BP-CORE-14` `AGENTS.md` surfaces each blueprint rule that must run at session start or before other task work as an executable line: rule identifier plus exact command. See `BP-INSTR-11`.
- `BP-CORE-15` Repositories apply the public-by-default and sensitive-content controls in `[BP-PUBLIC]`; `AGENTS.md` surfaces the pre-stage trigger.

**SHOULD**
- `BP-CORE-07` Keep policy lean; prefer references over duplicated rules. A rule that restates blueprint or `AGENTS.md` text verbatim is a FAIL in alignment reports. See `[BP-INSTR]`.
- `BP-CORE-08` Capture AI commit identity once per repo in `AGENTS.md` to avoid repeated prompts.
- `BP-CORE-10` Capture user interaction profile in `AGENTS.md` on project init or alignment.
- `BP-CORE-13` When a repository provides reusable skills, keep their canonical instructions in `skills/` and list their triggers and paths in `AGENTS.md`. See `[BP-SKILLS]`.

---

## Instruction Precedence [BP-PRECEDENCE]

When instructions conflict, resolve in this order (highest wins):

1. Explicit live user direction in the current session.
2. The active roadmap work unit's scope and specification.
3. `AGENTS.md` project policy. Agent wrapper files (`CLAUDE.md`, `GEMINI.md`, etc.) rank here when they carry agent-specific instructions.
4. `AGENT_BLUEPRINT.md` defaults.
5. Persistent memory and stored user profiles — background context only; they inform tone and defaults but never override the levels above.

Safety `[BP-SAFE]` is a gate, not a rank: destructive, irreversible, out-of-repo, or sensitive-publication actions still require confirmation even when a higher-precedence source requests them.

State precedence explicitly because unresolved instruction conflicts measurably reduce instruction-following ([IFScale], arXiv:2507.11538).

---

## Safety [BP-SAFE]

Confirm before running destructive commands, installing dependencies, taking actions outside the repo, or including sensitive content in repository history.

---

## Communication Standard [BP-COMM-STE]

Use an STE-based register for all agent-authored communication by default. This register applies to user replies, status updates, plans, recommendations, documentation, code comments, commit messages, PR descriptions, warnings, and errors.

- `BP-COMM-STE-01` Use a maximum of 20 words per instruction and 25 words per descriptive sentence.
- `BP-COMM-STE-02` Write one action per instruction and one topic per descriptive sentence.
- `BP-COMM-STE-03` Put each condition before its instruction.
- `BP-COMM-STE-04` Use one term for each concept.
- `BP-COMM-STE-05` Use `must` for requirements, `can` for permission or capability, and `will` for declared future actions.
- `BP-COMM-STE-06` State uncertainty as a fact about knowledge, evidence, or confidence. Do not use speculative modal verbs as substitutes for clear uncertainty.
- `BP-COMM-STE-07` Remove filler, idioms, rhetorical fragments, and unnecessary emphasis.
- `BP-COMM-STE-08` Preserve exact code, commands, identifiers, paths, quotations, product names, legal text, and external titles.
- `BP-COMM-STE-09` Use another register only when live user direction or a recorded project exemption requires it.

Examples of direct uncertainty:

- `The cause is not confirmed.`
- `The available evidence is incomplete.`
- `Confidence is low because the test did not reproduce the failure.`

This is an STE-based house register. Do not claim full ASD-STE100 compliance without validation against the complete standard and controlled dictionary.

Source: `references/sources.md` (`[18]`, `[19]`).

---

## Public Repository Safety [BP-PUBLIC]

Treat every repository as public unless `AGENTS.md` explicitly declares another visibility. Private visibility does not make credentials safe to commit.

### Default Ignore Policy [BP-PUBLIC-IGNORE]

At project initialization and alignment, ensure `.gitignore` covers these untracked local artifacts when relevant:

- `.env` and `.env.*`, with explicit exceptions for sanitized example files.
- Private keys, credential files, and local secret directories.
- `.agent-profile.md` and local agent memory or session transcripts that are not project documentation.
- `.private/` as the standard path for personal, unpublished, embargoed, or confidential content.
- Local exports, database copies, logs, backups, and generated files that can contain real user or production data.

Use project-specific paths when a broad wildcard can hide source artifacts. Do not ignore every draft path. Keep publishable drafts tracked, and place private drafts under `.private/`.

`.gitignore` does not protect files already tracked by Git. During alignment, flag tracked files that match these categories; do not remove or rewrite them without confirmation.

### Sensitive Content Check [BP-PUBLIC-CHECK]

Before staging or committing:

1. Inspect every candidate path and diff for secrets, personal data, unpublished or embargoed drafts, private correspondence, confidential business information, and real user or production data.
2. If live authentication material appears, stop and exclude it. Never commit or reproduce passwords, tokens, private keys, session cookies, or recovery codes. If the value can already exist in Git history, tell the user to revoke or rotate it.
3. For any other sensitive candidate, name the path, state the concern without quoting the sensitive content, and make exclusion the default.
4. Confirm inclusion of the named sensitive candidates separately from general commit approval.
5. Stage only reviewed paths. Do not use bulk staging while unreviewed files are present.

Use this confirmation format:

```text
Sensitive-content check:
- content/resume-draft.md — contains personal contact details and unpublished material.
Default: exclude this file.
Confirm inclusion of this file in a repository treated as public.
```

---

## Instruction Design [BP-INSTR]

How to author `AGENTS.md` and work units so agents actually follow them. Instruction-following accuracy declines as the number of active instructions rises ([IFScale]), and models attend most to the start and end of a file and least to the middle ([Lost in the Middle], arXiv:2307.03172). Write to those constraints:

- `BP-INSTR-01` Keep the active instruction set small. Split rules into layered files loaded on demand; a work unit must not restate blueprint or `AGENTS.md` rules. (density)
- `BP-INSTR-02` Order by importance. Put MUST invariants and precedence at the top of a file and easily-forgotten operational rules near the end; never bury load-bearing rules in the middle. (primacy/recency)
- `BP-INSTR-03` One instruction, one checkable outcome. Write each rule so compliance is verifiable; prefer concrete, testable criteria over adjectives. (reduces omission under load)
- `BP-INSTR-04` Prefer positive, specific instructions ("do X, with criterion Y"). Reserve prohibitions for named, recurring failure modes — e.g. the `Never Run` list — rather than blanket "don't." (positive + targeted-negative supervision)
- `BP-INSTR-05` Reference over restate. Link to the canonical rule instead of copying it; duplication raises density and drifts out of sync. (reinforces `BP-CORE-07`)
- `BP-INSTR-06` Write rules in the imperative with the trigger condition first: "When X, do Y." Condition-action phrasing makes the rule matchable at the moment it applies. (retrieval at point of use)
- `BP-INSTR-07` For any format agents must produce, include one filled-in example alongside the schema or template. A worked example constrains output better than field descriptions alone. (few-shot > schema)
- `BP-INSTR-08` Keep rationale out of the instruction stream. Justification aimed at humans (history, comparisons, persuasion) belongs in a README or companion doc; keep at most one line of "why" per rule. (density)
- `BP-INSTR-09` One word, one meaning. Use one term per concept across this blueprint, `AGENTS.md`, and work units. Three terms are fixed here: **validate** = run the project validation commands; **confirm** = get user approval; **check** = evaluate a stated condition. An agent reads three verbs as three operations. (vocabulary discipline)
- `BP-INSTR-10` Write requirements with `must`, `can`, or `will`. In a rule body, "should" reads as optional and "may/might/could" read as speculative. Write `must` for a requirement, `can` for a permission, or delete the rule. `SHOULD` stays valid as the normative label in `Core Invariants`. (removes hedge ambiguity)
- `BP-INSTR-11` Surface do-first triggers in `AGENTS.md` as executable lines. When a blueprint rule must run at session start or before other task work, `AGENTS.md` must state the trigger, carry the exact command inline, and include the rule identifier. Verification requires the identifier and the command to both be present; this trigger bridge is not a restatement under `BP-INSTR-05`. (availability at point of use)

Source for `BP-INSTR-09` and `BP-INSTR-10`: ASD-STE100 Simplified Technical English (Issue 9, 2025), adapted for agent instructions via [SimpleEnglish](https://github.com/AminBlg/SimpleEnglish) — AminBlg, MIT. `BP-INSTR-03` and `BP-INSTR-06` restate that standard's "one instruction per sentence" and "condition before command" rules, derived here independently. ASD-STE100 is a registered trademark of ASD; no specification or dictionary text is reproduced. Full bibliography: `references/sources.md` (`[18]`, `[19]`).

---

## Agent-Agnostic Skills [BP-SKILLS]

Use `skills/<skill-name>/SKILL.md` as the canonical location for reusable skill instructions.

- `BP-SKILLS-01` Write canonical skill instructions without model-provider or agent-client assumptions.
- `BP-SKILLS-02` List each shared skill's trigger and canonical path in `AGENTS.md`. This makes the skill available to agents that do not implement native skill discovery.
- `BP-SKILLS-03` Keep client-specific discovery paths, manifests, and UI metadata as adapters. They can point to or describe the canonical skill, but must not duplicate its workflow.
- `BP-SKILLS-04` When a task matches a listed trigger, read the canonical `SKILL.md` completely before acting. Read referenced resources only when the skill routes the task to them.
- `BP-SKILLS-05` Keep a skill self-contained. Put detailed tool mechanics in `references/`, deterministic automation in `scripts/`, and output templates or reusable media in `assets/`.

Use the shared skill by relative path when projects live in one workspace. Copy the skill into the consuming repository's `skills/` directory when the repository must remain portable outside that workspace.

---

## Vendor-Hosted Features [BP-VENDOR]

A vendor-hosted feature stores output in one vendor's service or works in only one vendor's client.
Examples: Claude Artifacts, ChatGPT Canvas, Gemini Canvas, hosted share links, and cloud-scheduled agents.
These features add lock-in and can publish content outside the repository.

- `BP-VENDOR-01` Deliver output as a terminal reply, a repository file, or an open format (Markdown, HTML, JSON, CSV, SVG).
- `BP-VENDOR-02` Use a vendor-hosted feature only when the user asks for it or confirms it in the current session.
- `BP-VENDOR-03` When a vendor-hosted feature can help, offer it in one sentence.
  Do not create it before the user confirms.
- `BP-VENDOR-04` An approval covers only the requested output.
  A project can record standing approval for a named feature in `AGENTS.md`.
- `BP-VENDOR-05` This rule overrides agent-client defaults that encourage proactive use.
  When a harness instruction conflicts, record the override in the agent wrapper per `[BP-AGENT-WRAPPER]`.

---

## Authored Artifacts [BP-WRITE]

Prose that an agent writes into the repository. `[BP-INSTR]` governs instruction files. `[BP-WF-PROFILE]` governs replies to the user. This section governs everything else.

- `BP-WRITE-01` Write documentation, code comments, commit messages, PR descriptions, and error text in the STE-based register defined in `[BP-COMM-STE]`.
- `BP-WRITE-02` Commit messages take an imperative subject line and a body in simple past. State what changed and why. Do not state intent ("this commit aims to").
- `BP-WRITE-03` Leave code, identifiers, file paths, and quoted error text exact. They are names, not prose.
- `BP-WRITE-04` Exempt human-facing persuasive text: launch posts, brand writing, and any marketing section of a `README`. The STE-based register removes persuasion by design. Record a project's exemptions in `AGENTS.md`.
- `BP-WRITE-05` Limit a commit subject to 50 characters. Limit a commit body to 3 sentences. Omit the body when the subject states the change completely.
- `BP-WRITE-06` Use semantic line breaks (`sembr.org`) in Markdown documentation.
  Apply to new and revised text only;
  do not reformat untouched paragraphs.
  Exempt commit messages and paste targets with block semantics, such as Roam.

Source: `references/sources.md` (`[18]`, `[19]`, `[27]`).

---

## Verifiability [BP-VERIFY]

An agent produces work faster than a user can check it. Users follow incorrect AI advice most of the time when checking is expensive (`[20]`, `[22]`), so lower the cost to check instead of adding friction — friction works, and users turn it off (`[21]`).

- `BP-VERIFY-01` Anchor every factual claim to something the user can check: `file:line`, the exact command, the diff, or the quoted output.
- `BP-VERIFY-02` Separate what you verified from what you inferred. Name the checks you ran and the checks you skipped.
- `BP-VERIFY-03` Report failures and gaps in full. State what you did not do, and why.
- `BP-VERIFY-04` Keep calibrated uncertainty. Do not convert `may` into `will` to sound decisive. False confidence suppresses user scrutiny (`[23]`).
- `BP-VERIFY-05` Before an irreversible or high-impact action, state the disconfirming case: what would make this the wrong move, and what the user can look at to decide. Gate this to the actions in `[BP-SAFE]`. Do not apply it to routine turns (`[21]`).

Numbered citations resolve in `references/sources.md`.

---

## Environment [BP-ENV]

Reproducible environments prevent "works on my machine" failures. Pin versions, commit lockfiles, and document setup.

### Version Pinning [BP-ENV-PIN]

If the project uses a language runtime, pin the version in a file committed to the repo. Use a format that version managers read automatically:

| Ecosystem | Version file | Manager(s) |
|-----------|-------------|------------|
| Node | `.nvmrc` or `.node-version` | nvm, fnm, volta, mise |
| Python | `.python-version` or `pyproject.toml` `[project.requires-python]` | uv, pyenv, mise |
| Rust | `rust-toolchain.toml` | rustup |
| Go | `go.mod` (`go` directive) | built-in |
| Clojure | `deps.edn` (`:deps` versions) | Clojure CLI |
| Bun | `package.json` `engines.bun` | bun |
| Multi-language | `.tool-versions` | mise, asdf |

### Lockfiles [BP-ENV-LOCK]

Commit lockfiles. They make dependency resolution deterministic across machines and CI.

Common lockfiles: `package-lock.json` / `bun.lockb` / `yarn.lock`, `uv.lock` / `poetry.lock`, `Cargo.lock`, `go.sum`, `deps-lock.json` (Deno).

If the ecosystem has a lockfile, commit it. When installing dependencies, use the lockfile-respecting command (e.g. `npm ci` not `npm install`, `uv sync` not `uv pip install`).

### Setup Command [BP-ENV-SETUP]

Document a single command (or short sequence) that bootstraps the environment from scratch. Store it in the `## Environment` section of `AGENTS.md` so agents can self-bootstrap.

### Web Server Port [BP-ENV-PORT]

For web app projects, the dev server must resolve its listening port with this precedence:

1. **Explicit override** (e.g. `--port`) — use it; if unavailable, error out. A user-specified port is a hard requirement, not a suggestion.
2. **Default port** — if no override, try the project's configured default; if unavailable, fall back to an open port.

Always print the chosen port so the user (and agent) knows where to connect. Document the default port and the override flag in the `## Environment` section of `AGENTS.md`.

---

## Workflow [BP-WF]

### Herdr Tab Naming [BP-WF-HERDR]

- `BP-WF-HERDR-01` After the first user prompt, when `HERDR_ENV=1` and `HERDR_TAB_ID` is set, rename the current tab before other task work.
- `BP-WF-HERDR-02` Prefix the label with one relevant emoji. Use the fewest words that identify the primary task, with a maximum of five words excluding the emoji.
- `BP-WF-HERDR-03` Run `herdr tab rename "$HERDR_TAB_ID" "<label>"` once per session.
- `BP-WF-HERDR-04` When Herdr is unavailable or the rename fails, continue without comment or another attempt.

Example:

```sh
herdr tab rename "$HERDR_TAB_ID" "🧭 Audit Payment Flow"
```

### Operating Model [BP-WF-OPS]

1. **Take direction** from a `roadmap/` work unit file, issue, or user request.
2. **If input is a brain dump**, create a draft work unit and clarify until scope and validation are concrete.
3. **Execute autonomously** once scope is clear; do not stop after each small step.
4. **Self-validate end-to-end** before returning: run required checks, create missing tests when needed, and run E2E for UI changes.
5. **Return to the user only when** done and validated, stuck, or blocked on an irreversible/high-impact decision.

### Validation [BP-WF-VAL]

Projects define validation commands in `AGENTS.md`:

- **Format/Lint** — run after every change
- **Build/Compile** — run after code changes
- **Unit tests** — run before declaring logic complete
- **E2E tests** — run after UI changes (start required services if approved)

Work through the validation hierarchy. Escalate only when lower levels pass.

### Guardrails [BP-WF-GUARD]

- Run validation after changes.
- Follow the execution policy defined in `AGENTS.md`.
- Keep changes minimal and focused; avoid unrelated improvements.
- For critical logic changes, review `git diff` before declaring completion.

### Learning Log [BP-WF-LEARN]

Projects can keep a `MISTAKES.md` evidence log for failures that can improve future work.

- `BP-WF-LEARN-01` After scoping a task, when `MISTAKES.md` exists, search it for entries relevant to the affected paths, systems, or operations before implementation.
- `BP-WF-LEARN-02` When an agent action causes an incorrect outcome, or the user corrects the agent, add a newest-first entry. Do not log expected exploration, rejected options, or failures outside the agent's control.
- `BP-WF-LEARN-03` Record the context, observed failure, consequence, root cause, prevention, related entries, and status. Mark an unverified root cause as `unknown`; do not convert a guess into policy.
- `BP-WF-LEARN-04` When multiple entries support the same verified prevention, promote it into the narrowest applicable canonical instruction. If policy changes are outside the current scope, report the promotion candidate instead.
- `BP-WF-LEARN-05` After promotion, mark the evidence entries `promoted` and link to the canonical instruction. Do not duplicate the promoted rule in the log.

This workflow is optional. When a project adopts it, surface the pre-implementation trigger in `AGENTS.md` under `Learning Log` per `BP-INSTR-11`.

Suggested `MISTAKES.md` entry:

```markdown
## YYYY-MM-DD — Short failure label

- Context: [task and affected area]
- Failure: [observable incorrect action or outcome]
- Consequence: [impact]
- Root cause: verified | unknown — [cause or missing evidence]
- Prevention: [specific action that would prevent recurrence]
- Related: [entry links or "none"]
- Status: active | promoted to `[canonical instruction]`
```

Source: `references/sources.md` (`[26]`).

### Visual Timeline [BP-WF-VISUAL]

Optional. Projects with visual UI and a screenshot or e2e harness can capture screenshot progressions during UI work. Keep two practices separate:

- **Series ticks** — one capture of every manifest route, building an ongoing progression timeline.
- **Before/after pairs** — per-change evidence for changelogs, reviews, and posts.

- `BP-WF-VISUAL-01` Before UI edits, capture a baseline series tick. A prior tick from the current source state counts. The user can waive the baseline.
- `BP-WF-VISUAL-02` When the working tree is dirty at baseline time, ask whether existing changes belong in the baseline.
- `BP-WF-VISUAL-03` Before committing UI changes, capture another series tick. Capture it even when no visible change is expected.
- `BP-WF-VISUAL-04` When capture needs a server the agent must not start, ask the user to run it. Wait for the result.
- `BP-WF-VISUAL-05` Keep ticks comparable: one canonical route manifest, fixed viewports, fixed wait conditions. Update the manifest in the same change as navigation changes.
- `BP-WF-VISUAL-06` Store each tick in one timestamped directory with a metadata sidecar: source revision, branch, dirty state, per-route capture status.
- `BP-WF-VISUAL-07` Prefix pair filenames with the phase (`before`/`after`) via environment variable or flag. Omit the prefix for CI captures.
- `BP-WF-VISUAL-08` Define the harness, output paths, and capture commit policy in `AGENTS.md`. The blueprint defines the contract; the project implements the mechanism.

When a project adopts this workflow, surface the baseline trigger in `AGENTS.md` per `BP-INSTR-11`. Skip for non-visual projects.

### Commits [BP-WF-COMMIT]

- Commit only after user approval.
- Before staging or committing, apply `[BP-PUBLIC-CHECK]`.
- Before committing, present: proposed commit message, files included, sensitive-content check result, and validation results.
- Write the message per `[BP-WRITE]`: imperative subject, body in simple past.
- Read the commit trailer template from `AGENTS.md`; if missing, ask once before the first commit in a repo.
- Never persist runtime values (`Co-authored-by`, `AI-Provider`, `AI-Product`, `AI-Model`) in `AGENTS.md`; fill them at commit time from session metadata.
- `AI-Model` must be the exact runtime model ID (for example, `gpt-5.6-sol`). Never use a family label such as `GPT-5`.
- Do not use the model's own statement of its identity as the `AI-Model` source. If no source gives the exact ID, ask the user.
- When filling trailers, resolve co-author identity, provider/model values, and multi-model attribution per `references/commit-attribution.md` (copied alongside this blueprint). When more than one model contributed, attribute all of them per that reference — never auto-add a second model without user confirmation.

### User Profile [BP-WF-PROFILE]

Calibrate agent interactions based on user context. Store in a git-ignored file (e.g., `.agent-profile.md`) referenced from `AGENTS.md`.

**Response calibration (default):** Use the concise STE-based register in `[BP-COMM-STE]` for all responses. Expand only when the user asks or the requested artifact requires detail. Lead with the conclusion, support after. Treat the user's message as a premise to build from, not a statement to evaluate, rate, or reflect back. Disagree openly when warranted; do not hedge or amplify to be agreeable. Store per-user specifics (length contract, mode triggers, explanation depth, domains) in the profile file, not here.

**Response style (default):** Start with substance. Do not praise the user's framing before engaging, restate it with inflated importance, or mirror emotion performatively. Use direct prose. Avoid canned transitions, rhetorical fragments, contrastive reframes ("not X, but Y"), manufactured emphasis, decorative three-part lists, and excessive em dashes. Use headings only when they improve navigation. Do not repeat the conclusion or end with a summary or offer unless requested. Never open with "You're absolutely right," "Great question," "Let's unpack this," "Here's the thing," or "It's worth noting." Skip pleasantries, hype, and apologies except when correcting an error.

**Register (default):** Length and register are independent axes. An expansion request changes length, not register. Apply `[BP-COMM-STE]` to factual and deliberative passages. State uncertainty as a fact about knowledge, evidence, or confidence. Use another register only when live user direction or a recorded project exemption requires it.

Precedence for response calibration: this default < `.agent-profile.md` < live conversation. (See `[BP-PRECEDENCE]` for the full ladder.)

**Prompting conditions** (check at project initialization and on alignment runs):
1. **No profile exists** → Prompt to create one
2. **Profile exists but incomplete** (missing fields from current blueprint guidance) → Prompt to fill gaps
3. **Profile complete** → Ask if user wants to update

Profile dimensions, interview questions, and calibration guidance live in `references/user-profile.md` (copied alongside this blueprint); load it only when creating or updating a profile.

---

## Adoption [BP-ADOPT]

1. Copy this file as `AGENT_BLUEPRINT.md`.
2. Copy the `references/` directory alongside it (commit attribution, user profile guidance, work unit example, sources).
3. Create `AGENTS.md` using the template below.
4. Create `roadmap/index.md`.
5. Create or update `.gitignore` using `[BP-PUBLIC-IGNORE]`.
6. Optionally create `MISTAKES.md` using `[BP-WF-LEARN]` and add its trigger bridge to `AGENTS.md`.
7. For visual UI projects, optionally adopt `[BP-WF-VISUAL]` and add its trigger bridge to `AGENTS.md`.
8. Optionally create agent-specific wrappers (`CLAUDE.md`, `GEMINI.md`, etc.) using the wrapper template.

Agent-specific files (`CLAUDE.md`, `GEMINI.md`, etc.) are optional. When you create one, keep it a thin pointer to `AGENTS.md`.

---

## Versioning [BP-VERSION]

Use date-based versions, not semantic versioning.

**Format:** `YYYY-MM-DD` with an optional `.N` suffix for same-day releases.

```
2026-03-07        ← first release of the day
2026-03-07.1      ← second release same day
2026-03-07.2      ← third, etc.
```

Date versions are honest, monotonically increasing, and require zero decision overhead. (Fuller rationale: `README.md`.)

**Rules:**
- The frontmatter `version` field in this blueprint and companion documents uses this scheme.
- `AGENTS.md` and other files that reference the blueprint version must carry the same date string.
- When you adopt this blueprint in a new project, use date-based versioning. A team with an existing convention can keep it, and must record that choice in `AGENTS.md`.
- Do not debate version bumps. Update the date, move on.

---

## Alignment Contract [BP-ALIGN]

- `AGENTS.md` is the project policy entrypoint and references this blueprint.
- `roadmap/` is the canonical place for scoped work units and execution prompts.
- A `ready` work unit is executable without additional clarification.
- Keep policy lean: prefer references over duplicated instructions.
- A blueprint reference alone does not satisfy `BP-INSTR-11`; `AGENTS.md` must surface each do-first trigger.

### Align Project With This Blueprint

When asked to align a project:
1. Compare `AGENTS.md` and `roadmap/` against this blueprint, including each `BP-INSTR-11` do-first trigger bridge.
2. Report gaps and propose a minimal patch plan.
3. Apply focused edits and run project validation commands.
4. Return with completed changes plus any remaining questions.

### Required Alignment Report Format [BP-ALIGN-REPORT]

Use this format exactly:

```markdown
# Alignment Report

## Blueprint
- Version: [e.g. 2026-03-07]

## Rule Check
| Rule ID | Status (PASS/FAIL) | Evidence | Action |
|---|---|---|---|
| BP-CORE-01 | PASS | `AGENTS.md` references blueprint | n/a |

## Patch Plan
1. [minimal change]
2. [minimal change]

## Applied Changes
- `[file path]`: [what changed]

## Validation
- `[command]`: [pass/fail + brief output]

## Open Questions
- [only unresolved decisions]
```

---

## AGENTS.md Template [BP-AGENTS-TPL]

````markdown
# AGENTS

Follows `AGENT_BLUEPRINT.md` (version: [BLUEPRINT_VERSION])

## Session Start

- Herdr tab naming `[BP-WF-HERDR]`: before other task work, when `HERDR_ENV=1` and `HERDR_TAB_ID` is set, run once `herdr tab rename "$HERDR_TAB_ID" "<label>"` (`<label>` = one relevant emoji + at most five words naming the primary task); on failure, continue silently without retry.

## Project Overview

[One paragraph: what this is, language/framework, key domains.]

## Repository Visibility

- Visibility: public | private (defaults to public when unspecified)
- Before staging or committing, apply `AGENT_BLUEPRINT.md` `[BP-PUBLIC]`.

## Stack

- [Language + version]
- [Framework/runtime]
- [Database]
- [Infra/deploy target]

## Environment

- Version manager: [e.g. uv, nvm, mise, rustup, or "built-in"]
- Version file: [e.g. `.python-version`, `.nvmrc`, `rust-toolchain.toml`]
- Lockfile: [e.g. `uv.lock`, `package-lock.json`, `Cargo.lock`]
- Setup: `[single bootstrap command, e.g. "uv sync", "npm ci", "cargo build"]`

## Commit Trailer Template

Store a template, not concrete runtime values. Fill it at commit time using the resolution rules in `references/commit-attribution.md`.

```text
Co-authored-by: [AI_PRODUCT_NAME] <[AI_PRODUCT_EMAIL]>
AI-Provider: [AI_PROVIDER]
AI-Product: [AI_PRODUCT_LINE]
AI-Model: [AI_MODEL]
```

Write the trailer lines consecutively. A blank line between trailers stops `git interpret-trailers` from parsing the lines above it.

`[AI_MODEL]` is the exact runtime model ID (for example, `gpt-5.6-sol`), never a family label such as `GPT-5`.

## Validation Commands

| Level | Command | When |
|-------|---------|------|
| 1 | `[format/lint]` | After every change |
| 2 | `[build/compile]` | After code changes |
| 3 | `[test]` | Before completing work |
| 4 | `[e2e]` | After UI changes |

## Execution Modes

- `roadmap/` is the canonical planning surface.
- If roadmap work unit files use numeric IDs, document the digit width used by this repo in `AGENTS.md` (blueprint default: 3).
- Validation commands are defined above and applied when relevant.
- Keep changes minimal and scoped to the requested work unit.
- Require user confirmation before `git commit`, installs, upgrades, or network calls with external side effects.
- Before using a vendor-hosted feature (for example, Claude Artifacts), confirm with the user unless they asked for it. Apply `AGENT_BLUEPRINT.md` `[BP-VENDOR]`.
- It is acceptable to stop for clarification when scope is ambiguous.

## Never Run

- `[command]` — [why]

## Project-Specific Rules

- [constraints, data sensitivity, architectural boundaries]
- [`BP-WRITE-04` exemptions: files or sections that carry persuasive voice, or "none"]

## Skills (optional)

- When [trigger], read and follow `[path]/SKILL.md` before acting.
- Treat `[path]/SKILL.md` as canonical; client-specific skill metadata is only a discovery adapter.

## Learning Log (optional)

- When `MISTAKES.md` exists, after scoping a task, search it for relevant prior failures before implementation. Apply `AGENT_BLUEPRINT.md` `[BP-WF-LEARN]`.

## Visual Timeline (optional)

- When work touches UI, before edits, capture a baseline series tick. Apply `AGENT_BLUEPRINT.md` `[BP-WF-VISUAL]`.

## Decision Artifacts

- For high-impact or irreversible decisions, record a decision matrix in `.decisions/[name].json`.
- Use `matrix-reloaded` format for structured comparison.
- Do not run `matrix-reloaded` CLI commands from agent sessions; use project-provided matrix instructions/schema.
- Optional: add `.decisions/[name].md` for human-readable narrative context.
- Treat the JSON decision matrix as the authoritative record.

## References

- For [topic], see `[doc path]`
- For decision records and optional matrix format, see `AGENT_BLUEPRINT.md` section `Decision Artifacts [BP-DECISIONS]`.

## Key Files

- `[path]` — [purpose]

## User Profile (optional)

See `.agent-profile.md` (git-ignored) for interaction preferences. Create on project init or alignment.

## Response Style

Before every user reply, apply `AGENT_BLUEPRINT.md` `[BP-WF-PROFILE]`.
````

---

## Agent-Specific Wrapper Template [BP-AGENT-WRAPPER]

Optional. Create thin pointers for agent-specific entrypoints (`CLAUDE.md`, `GEMINI.md`, etc.).

A wrapper MUST make the harness **load** `AGENTS.md`, not merely mention it. Prose
like "See `AGENTS.md`" is a request the agent may never act on, and the harness
reads only its own wrapper file. Use the harness's import mechanism where one
exists; fall back to prose only where none does.

`CLAUDE.md` — Claude Code expands `@path` imports at session start:

```markdown
@AGENTS.md

## Agent-Specific Instructions

- [Instruction specific to this agent, if any]
- [e.g., tool preferences, model-specific behavior, constraints]
- [Override of a conflicting built-in harness instruction, if any]
```

The `@AGENTS.md` line MUST be unquoted and outside any code fence. Claude Code
skips import parsing inside code spans and fenced blocks, so `` `@AGENTS.md` ``
is inert — it silently loads nothing. Omit the `# [Agent Name]` heading; the
import must be the file's own content.

Other harnesses: use the equivalent import if the harness documents one, and
verify it loads before relying on it. Where none exists, keep the prose pointer:

```markdown
# [Agent Name]

See `AGENTS.md` for project policies and operating rules.

## Agent-Specific Instructions

- [Instruction specific to this agent, if any]
```

Keep minimal. Defer to `AGENTS.md` for all shared policy.

Verify after writing: start the agent and confirm the wrapper resolved (in Claude
Code, `/context` lists `CLAUDE.md` under **Memory files**). An import is
load-bearing and fails silently — if `AGENTS.md` is renamed or moved, the wrapper
becomes empty and every session loses all project rules with no error.

A wrapper can override a built-in instruction in its host agent harness. Use this only when the built-in instruction conflicts with `AGENTS.md`. Name the conflicting instruction and state the required behavior. A pointer to `AGENTS.md` does not by itself displace a harness default.

---

## Roadmap [BP-RM]

This is the core execution model. Work units are prompts for autonomous agent work.

### Structure

```
roadmap/
├── index.md       # Project overview and directory of work units
├── _template.md   # Starting point for new work units
├── [ID]-[slug].md # Individual work unit files (with frontmatter)
└── archived/      # Completed or dropped work units
```

Non-work-unit helper files such as `index.md` and `_template.md` remain unnumbered.

### Work Unit Filenames [BP-RM-FILES]

Roadmap work unit files must use `[ID]-[slug].md`.

- `ID` is a stable numeric identifier used for reference and sorting only.
- Assign IDs sequentially and never change them once assigned.
- IDs do not encode priority, status, or anything beyond initial creation-order assignment.
- Zero-padding is required for lexical sorting.
- Default width is 3 digits.
- A repo can use a different digit width, and must record that width in `AGENTS.md`.

### Numbering Alignment Guidance [BP-RM-FILES-ALIGN]

When adopting numbered work unit filenames in an existing repo:

1. Assign IDs by `created` date when present.
2. If `created` is missing, preserve the current logical or file order.
3. Rename work unit files in both `roadmap/` and `roadmap/archived/`.
4. Update internal references after renaming.
5. Do not renumber existing work units after IDs are assigned.

### Work Unit Frontmatter [BP-RM-FRONTMATTER]

Every work unit file **must** begin with YAML frontmatter for machine parsing:

```yaml
---
title: "Feature Name"
status: draft | ready | active | done | dropped
description: "One-line summary of what this work unit accomplishes"
created: 2024-01-15
updated: 2024-01-20
tags: []
priority: medium                      # high | medium | low
---
```

**Required fields:**
- `title` — Display name for the work unit
- `status` — Current state (see Status Definitions below)
- `description` — One-line summary
- `created` — Date work unit was created (YYYY-MM-DD)
- `updated` — Date of last modification (YYYY-MM-DD)
- `tags` — Array for categorization and filtering
- `priority` — high | medium | low (default: medium)

### Status Definitions

| Status | Meaning | Kanban Column |
|--------|---------|---------------|
| `draft` | Brain dump captured; has open questions | Backlog |
| `ready` | Clarified and executable as-is | Backlog |
| `active` | Currently being worked on | In Progress |
| `done` | Shipped and working | Done |
| `dropped` | Decided not to pursue | (hidden) |

### Legacy Status Migration [BP-RM-MIGRATION]

When aligning older projects:

| Legacy Status | New Status | Migration Rule |
|---|---|---|
| `idea` | `draft` | Keep open questions in `Open Questions`. |
| `planned` | `ready` | The Definition of Ready checklist must pass. |
| `paused` | `active` | Keep status `active` and add blocked context in `Context`. |
| `done` | `done` | No change. |
| `dropped` | `dropped` | No change. |

### Definition of Ready [BP-RM-DOR]

A work unit can be marked `ready` only if all are true:
- `Intent` states what and why.
- `Specification` is concrete and testable.
- `Validation` includes concrete checks (tests/e2e/visual as relevant).
- `Scope` explicitly defines boundaries.
- `Context` points to key files/constraints.
- `Open Questions` is empty or removed.

If any item is missing, status must remain `draft`.

### index.md Template

```markdown
---
title: "Project Name Roadmap"
goal: "One sentence: what this project exists to achieve."
---

# Roadmap

## Current Focus

[What is actively being worked on right now.]

## Work Units

See individual `[ID]-[slug].md` files in this directory. Use `draft` during clarification and `ready` when autonomous execution can begin.

## Quick Ideas

Ideas not yet promoted to work units:

- [Idea that doesn't need a file yet]
- [Another idea]
```

### _template.md

```markdown
---
title: "Work Unit Title"
status: draft | ready | active | done | dropped
description: "One line"
created: YYYY-MM-DD
updated: YYYY-MM-DD
tags: []
priority: high | medium | low
---

# Work Unit Title

## Intent

[What this accomplishes and why it matters.]

## Specification

[Concrete description of the change. What exists after this is done.]

## Validation

[How to know it's done:]
- [ ] Tests to create/pass
- [ ] E2E flows to run
- [ ] Visual criteria (reference style guide if applicable)

## Scope

[What's not included. Boundaries to prevent drift.]

## Context

[Pointers to relevant files, prior decisions, or constraints.]

## Open Questions (draft only)

[Unresolved items. Clear this section before moving to ready.]
```

### Example

A filled-in `ready` work unit lives at `references/work-unit-example.md` (copied alongside this blueprint). Match its concreteness — especially in `Specification` and `Validation` — when promoting a work unit from `draft`.

### Brain Dump to Ready Workflow

When creating a new work unit from a brain dump:
1. Create the file with status `draft`.
2. Ask clarifying questions until scope and validation are concrete.
3. Do not extrapolate uncertain requirements; ask instead.
4. Once questions are resolved, update status to `ready`.
5. A `ready` work unit must be a complete prompt an agent can execute without further clarification.

### Rules

- `roadmap/index.md` existence identifies a compatible project.
- Every work unit file must have valid YAML frontmatter.
- Status lives in frontmatter, not in prose.
- Keep work units concrete enough to execute and validate.
- When a work unit reaches `done` or `dropped`, move the file to `archived/`.
- Update the `updated` field whenever you modify a work unit.

---

## Decision Artifacts [BP-DECISIONS]

Optional. Use for high-impact or irreversible decisions, or when revisiting the same decision.

### Structure

Every decision has a JSON matrix file. Optionally add a markdown companion for narrative context.

```
.decisions/
├── database-choice.json     # Required: authoritative matrix-reloaded decision record
├── database-choice.md       # Optional: human-readable summary
└── auth-strategy.json
```

### JSON Matrix (required)

For each decision, add a `.json` file using `matrix-reloaded` format. Do not execute `matrix-reloaded` CLI commands from agent sessions; use project-provided instructions/schema for the expected JSON structure. The JSON matrix is the authoritative decision record.

### Markdown Format (optional)

```markdown
# Decision: [Title]

**Status:** proposed | accepted | superseded | rejected
**Date:** YYYY-MM-DD

## Context

[Why this decision is needed.]

## Options

### Option A
- Pros: [...]
- Cons: [...]

### Option B
- Pros: [...]
- Cons: [...]

## Decision

[What was chosen and why.]

## Consequences

[What changes. What to watch for.]
```

---

## Knowledge Base Integration [BP-KB]

Optional. Use this for projects that capture AI-generated summaries in an external knowledge tool (Roam Research, Obsidian, Notion, etc.).

### Enable in AGENTS.md

Add a `## Knowledge Base` section to `AGENTS.md` with tool-specific conventions. When present, agents generate structured output ready to paste into the user's knowledge base.

### Thread Summary Format

The `roam-thread-summary` skill (`skills/roam-thread-summary/`) is the canonical generator; it emits a paste-ready block. Its required parent attribution block is:

1. **Thread marker** — `[[ai-thread]]`
2. **Model** — `[[<model-id>]]`, the exact model of the current session
3. **Project** — `[[<project>]]`, the project's declared Roam tag or repository name
4. **Tool** — `[[<tool>]]`, the agentic harness the session runs in (`[[opencode]]`, `[[claude-code]]`, `[[gemini-cli]]`, `[[codex-cli]]`)

Add optional topic-page refs only when the user asks.

### Roam Research Example

Store in `AGENTS.md`:

```markdown
## Knowledge Base

Tool: Roam Research

When asked to generate a Roam summary or thread, use the `roam-thread-summary` skill:
- Required parent block: `- [[ai-thread]] [[<model-id>]] [[<project-name>]] [[<tool>]]`
- Optional refs (only if instructed): topic pages
- Sections: ask user what they want (chronological, functional, Q&A)
```

Output structure:

```
- [[ai-thread]] [[glm-5]] [[agent-blueprint]] [[opencode]]
    - Summary
        - Investigated stale cache issue in `src/cache.ts:142`
    - Files Changed
        - `src/cache.ts` - added TTL validation
    - Next Steps
        - Consider integration tests for cache invalidation
```

### Other Tools

Adapt the format for tool conventions:
- **Obsidian**: Use `#tags` and `[[wikilinks]]` with YAML frontmatter if desired
- **Notion**: Use nested bullet structure with database-compatible formatting
- **Logseq**: Similar to Roam with `[[bracket]]` syntax

---

## Design System [BP-DESIGN]

For projects with visual UI, use `DESIGN_SYSTEM_GUIDE.md` to establish consistent interface patterns.
The guide must use concrete, testable values (tokens/patterns), not only subjective descriptions.

If this project requires visual design and no design system exists:
1. Ask the user if they want to establish a design system.
2. If yes, copy `DESIGN_SYSTEM_GUIDE.md` into the project.
3. Follow its workflow to capture decisions in `.interface-design/system.md`.

Skip for CLI tools, libraries, backends, or other non-visual projects.
