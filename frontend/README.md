# Observatory Frontend

This directory contains the Svelte control room and its Tauri desktop bridge.
The Rust backend remains responsible for JSONL inspection, parsing, session
routing, parent-session lookup, and Worklog generation.

## Transcript data flow

The initial load requests all observation kinds once and retains those
transcript blocks in frontend state. Filter changes project the retained blocks
without reparsing the source file. Theme changes select the corresponding
renderer without an additional keyed remount of the transcript container.

Collapse state is owned by `App.svelte` and is reset when the transcript scope
(source session, filter, or theme) changes. **Capture Transcript** serializes the
same filtered blocks, references, theme, and collapse state directly from data;
it does not scrape rendered DOM text.

## Update availability

At launch, `App.svelte` reads the packaged application version and performs one
unauthenticated request to GitHub's latest public release endpoint. A notice is
passed to the three initial transcript presentations only when the returned
stable version is newer. Network failures and unrecognized responses remain
silent, and the fixed official Releases URL is the only update link opened.

## Development

Install the locked dependencies and start an interactive desktop development
run from `frontend/`:

```text
npm ci
npm run tauri:dev
```

For automated tests, static checks, production builds, Rust package tests,
desktop build verification, and change-specific requirements, follow the
[test matrix](../docs/test-matrix.md).

The desktop command layer is under `src-tauri/`. It adapts typed backend DTOs to
Tauri commands and must not duplicate parser or session-routing behavior.
