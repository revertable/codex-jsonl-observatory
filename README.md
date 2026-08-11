# Codex JSONL Observatory

Codex JSONL Observatory is a local desktop tool for reading Codex session JSONL files and exporting versionable worklog bundles. Supported session sources include Codex CLI and the Codex Windows app. Built with Rust, Svelte, and Tauri, it processes session data locally.

Use it for transcript reading, inspecting raw entries and parser diagnostics, or turning a complete session into structured files that can live with a project workspace, documentation, internal archive, or repository.

## Lineage and release status

Codex JSONL Observatory is the second-generation successor to [Codex Chat Viewer](https://github.com/revertable/codex-chat-viewer), the earlier tool in this product line. It continues the same problem space of reading Codex session JSONL files while rebuilding the workflow as a Rust/Svelte/Tauri local desktop app.

`v0.1.0` is the first public Windows portable release of Codex JSONL Observatory. It is a functional release covering the current product workflow described in [Features](#features), [Reading a session](#reading-a-session), and [Export Worklog](#export-worklog).

Just download the Windows portable zip, unzip it, and run the app. No server setup, cloud account, or developer environment is required.

## Features

- Open a Codex session JSONL file from Codex CLI or the Codex Windows app with the file picker or a local path.
- Read parsed transcript blocks in **Terminal Style**, **Markdown Style**, **DM Style**, or **DM Style (Dark)**.
- Focus the transcript with role filters for You, Codex, tool calls, tool results, and metadata.
- Inspect paginated raw entries, the resolved source path, parser counters, and observed event counts under **Raw Entries & Diagnostics**.
- Copy the detected `codex resume <session-id>` command with **Copy Resume Command**.
- Open the related [Cosmic Horizon Archive](https://riu-salze-studio.gitbook.io/cosmic-horizon) with **Visit Cosmic Horizon**.
- Export the complete session as a versionable worklog bundle with **Export Worklog**.

## Reading a session

Use **Select JSONL** to choose a Codex session JSONL file from Codex CLI or the Codex Windows app. You can also paste a local JSONL path and use **Refresh** to load or reload it.

The main transcript presents parsed blocks in the selected reading theme. Role filters change what appears in this view without changing the source session. Open **Raw Entries & Diagnostics** when you need the entry-level representation or parsing details.

When a session ID is available, **Copy Resume Command** copies the corresponding Codex CLI resume command to the clipboard.

## Export Worklog

**Export Worklog** turns a session into a folder bundle organized around the requests that drove the work. Choose an export parent directory and the app creates a bundle with this shape:

```text
<selected-parent>/
└─ codex-worklog/
   └─ YYYY-MM-DD/
      └─ HHMMSS_<source-id>/
         ├─ 000_index.md
         ├─ 001_HHMMSS.md
         ├─ 002_HHMMSS.md
         ├─ ...
         └─ manifest.json
```

Each `[YOU]` block starts a work unit. Following Codex/assistant responses, tool calls, tool results, and report messages remain in that unit until the next `[YOU]` block. `000_index.md` describes the source session and links the numbered work-unit files; `manifest.json` records the generated bundle and supports safe, compatible refreshes.

Export always uses the full source session, not the currently filtered transcript view. Re-exporting the same compatible session bundle refreshes its generated files through `manifest.json`. After a successful export, the app opens the generated bundle folder in the operating system's file explorer.

Directory and file names use local, user-facing time. Original timestamps remain in generated content and metadata where they are available.

> Exported worklogs may contain prompts, local paths, command output, code snippets, and project-specific details. Review a bundle before sharing it.

## Runtime and development

The application uses a Svelte frontend in a Tauri desktop shell. Tauri calls the Rust parser and export boundary directly for local JSONL processing.

Development and verification commands run from `frontend/`:

```text
npm run check
npm run build
npm run tauri:dev
npm run tauri:build
```
