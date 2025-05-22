# Cution

## Description

Experimental Curator project without hand coding, which is made by only Codex or Background Agent.
Trust Vibe.

## Features

- Fetch top stories from Hacker News API
- Extract article text content
- Generate summaries using LLM
- Store Markdown files in Supabase Storage
- Fetch content from a custom site defined by `CUSTOM_SITE_URL`

## Setup

1. Set required environment variables
   - `SUPABASE_URL`: Your Supabase project URL (e.g., `https://your-project-id.supabase.co`)
   - `SUPABASE_ANON_KEY`: Supabase Anonymous Key (or `SUPABASE_SERVICE_ROLE_KEY`)
   - `SUPABASE_BUCKET_NAME`: Supabase Storage bucket name (e.g., `cution`)
   - `GEMINI_API_KEY`: Google Gemini API Key
   - `CUSTOM_SITE_URL`: URL of the website you want to fetch
   - `XAI_API_KEY`: xAI API Key used for live search

2. Build and run
   ```
   cargo build --release
   ./target/release/orchestrator
   ```

## Deploy to Render

1. Push your repository to GitHub
2. Create a new Cron Job in Render
   - Runtime: Rust
   - Build command: `cargo build --release`
   - Start command: `./target/release/orchestrator`
   - Schedule: Set the time for daily execution (e.g., `0 8 * * *`)
3. Configure environment variables in the Render dashboard

## Supabase Configuration

1. Create a Supabase project
2. Create a new bucket in the "Storage" section of the Supabase dashboard
   - Make the bucket Public or set up RLS policies to allow access from the `anon` key (or `authenticated` role) as needed
   - Ensure proper write permissions for the upload directory (e.g., `cution`)
   - (Optional) Create a new bucket with pnpm
   ```
   pnpm i
   pnpm setup
   ```
