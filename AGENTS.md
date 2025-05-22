# AGENTS.md — Repository-wide Agent & Contributor Guide
This file defines conventions and mandatory checks for **every** file in this repository (`/`).
Nested `AGENTS.md` files may override rules for their sub-trees.

---

## 1 Scope
The scope of this file is the entire repository (`/`). All agents/humans touching any file must comply.

---

## 2 Coding Conventions
• Language : Rust 2021 edition (all crates in `/crates/*`).
• Formatting : `cargo fmt --all` (must report no diff).
• Linting : `cargo clippy --all -- -D warnings` (deny warnings).
• Error handling : prefer `anyhow::Result<T>` and `?` propagation; avoid `unwrap()`/`expect()` in library code.
• Module layout : public API in `lib.rs`; binaries in `main.rs`.
• Concurrency : use `tokio` multi-thread runtime; spawn tasks via `tokio::spawn` or `JoinSet`.

---

## 3 Workspace Layout
```
/crates
  ├── common         # shared utilities & clients
  ├── <feature-crate> # e.g. github, hacker_news …
  └── orchestrator   # top-level binary coordinating crawlers
```
• All new crates live under `/crates/` with their own `Cargo.toml`.
• Shared code goes to `crates/common`. Duplicate logic is prohibited.

---

## 4 Environment Variables
Required keys (fail fast if missing):

| Key | Purpose |
|-----|---------|
| `SUPABASE_URL` | Supabase project URL |
| `SUPABASE_SERVICE_ROLE_KEY` | Service-role API key |
| `SUPABASE_BUCKET_NAME` | Storage bucket |
| `XAI_API_KEY` | xAI API |
| `GEMINI_API_KEY` | Gemini LLM |
| `CUSTOM_SITE_URL` | Target site URL |
| `LANGUAGES` | Comma-separated GitHub trending languages |

---

## 5 Git & Workflow Rules
1. **Do NOT create new branches** during automated tasks.
2. Commit with `git add` → `git commit -m "…"`.
3. Ensure `git status` is clean before finishing.
4. If *pre-commit* hooks exist, they must pass; fix and retry.
5. Never amend or rebase existing commits created by previous agents.

---

## 6 Mandatory Programmatic Checks
Before committing, run from repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all -- -D warnings
cargo test --all
```
All must succeed.

---

## 7 Citation Rules
When responding to prompts:

• Cite files: `【F:<path>†L<start>(-L<end>)?】`
• Cite terminal chunks: `【<chunk_id>†L<start>(-L<end>)?】`
• Prefer file citations over terminal unless testing output is essential.
• Line numbers must be accurate; do not cite empty lines or commit hashes.

---

## 8 Pull-Request (PR) Message Guidelines
If generating a PR message, include:

1. **Summary** – high-level description.
2. **Changes** – bullet list of key modifications with file citations.
3. **Rationale** – why change is needed.
4. **Checks** – mention that `cargo fmt/clippy/test` passed.

Follow any additional instructions from more-nested `AGENTS.md` files.

---

## 9 Adding New Functionality
1. Create a new crate under `/crates/` if the concern is independent.
2. Add dependency path entries to `orchestrator/Cargo.toml` when orchestrated.
3. Update orchestrator logic to spawn the new crawler using `JoinSet`.
4. Ensure environment variables are documented above.

---

## 10 Change Log
| Date | Author | Description |
|------|--------|-------------|
| 2024-04-XX | initial | First version |

## 11 tasks
Task Tracking:
- Review `tasks.yaml` in the repository root before starting work.
- Apply listed tasks and mark them completed in `tasks.yaml`.