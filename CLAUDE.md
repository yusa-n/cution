# Claude Code Configuration

## Project Overview
This is a Rust workspace project called "vibe-cution" that appears to be a data aggregation system with multiple crates for different sources.

## Crates
- `arxiv/` - ArXiv data source
- `common/` - Shared utilities and Supabase client
- `custom_site/` - Custom site integration
- `github/` - GitHub data source
- `hacker_news/` - Hacker News data source
- `orchestrator/` - Main orchestration service
- `xai_search/` - XAI search functionality

## Development Commands
```bash
# Build all crates
cargo build

# Run tests
cargo test

# Check code
cargo check

# Format code
cargo fmt

# Run clippy linting
cargo clippy
```

## Key Files
- `Cargo.toml` - Workspace configuration
- `tasks.yaml` - Task definitions
- `scripts/setup_bucket.ts` - Bucket setup script