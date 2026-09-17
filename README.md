# Codex JSONL Observatory

**Language:** English | [한국어](README.ko.md)

Codex JSONL Observatory is a local desktop tool for reading Codex session JSONL files and exporting versionable worklog bundles. Supported session sources include Codex CLI and the Codex Windows app. Built with Rust, Svelte, and Tauri, it processes session data locally.

Use it for transcript reading, capturing a filtered transcript as text, or turning a complete session into structured files that can live with a project workspace, documentation, internal archive, or repository.

## Lineage and release status

Codex JSONL Observatory is the second-generation successor to [Codex Chat Viewer](https://github.com/revertable/codex-chat-viewer), the earlier tool in this product line. It continues the same problem space of reading Codex session JSONL files while rebuilding the workflow as a Rust/Svelte/Tauri local desktop app.

`v1.1.1` is the current public Windows portable release. It sharpens transcript focus with conversation-first default filters, adds a quick return to the top, and removes redundant terminal spacing.

Just download the Windows portable zip, unzip it, and run the app. No server setup, cloud account, or developer environment is required.

## Features

- Open a Codex session JSONL file from Codex CLI or the Codex Windows app with the **Select JSONL** file picker.
- Read parsed transcript blocks in **Terminal Style**, **Markdown Style**, **DM Style**, or **DM Style (Dark)**.
- Focus the transcript with role filters for You, Codex, tool calls, tool results, and metadata.
- Distinguish human-authored requests from recognized ChatGPT-to-Work handoff data and injected ambient UI context.
- Show referenced ChatGPT conversation provenance by title and conversation ID without repeating its cached preview in the transcript.
- Identify Guardian review sessions as internal specialized sessions and open a locally available parent session by verified thread ID.
- Capture the currently filtered and themed transcript as clipboard text with **Capture Transcript**.
- Refresh the selected session from either the top controls or the actions below the transcript.
- Copy the detected `codex resume <session-id>` command with **Copy Resume Command**.
- Open the related [Cosmic Horizon Archive](https://riu-salze-studio.gitbook.io/cosmic-horizon) with **Visit Cosmic Horizon**.
- Export the complete session as a versionable worklog bundle with **Export Worklog**.

## Reading a session

Use **Select JSONL** to choose a Codex session JSONL file from Codex CLI or the Codex Windows app. The selected path is displayed as read-only, and **Refresh** rereads the currently selected file from disk.

The main transcript presents parsed blocks in the selected reading theme. Role filters change what appears in this view without changing the source session. **Capture Transcript** copies the text currently displayed in the transcript, including the filtered blocks. A second **Refresh** action below the transcript reloads the selected session without requiring you to scroll back to the top. Click the **loaded** status to clear the selected session and return the app to its initial idle state.

When a session ID is available, **Copy Resume Command** copies the corresponding Codex CLI resume command to the clipboard.

### Human requests and injected context

A JSONL payload with `role=user` does not always represent text typed by a person. ChatGPT-to-Work sessions may include referenced-conversation metadata, a generated delegated task, and automatically supplied browser or runtime context in the same role.

Observatory conservatively recognizes the confirmed transport envelopes. A delegated `Continuing from ...` task is hidden only when its title and `chatgpt-conversation://` ID exactly match the parsed reference metadata. For a confirmed `ambient-ui-state` envelope, only the text after `## My request:` is rendered as `[YOU]`. Unrecognized or incomplete structures fall back to their original text instead of being discarded.

Referenced ChatGPT conversations appear once as session provenance with their title and conversation ID. The bounded `priorConversation` cache is not reproduced in the default transcript. The source JSONL file is never rewritten.

### Specialized sessions

Guardian review rollouts are shown as internal specialized sessions rather than as ordinary `[YOU]`/`[CODEX]` conversations. Resume and Worklog export are unavailable for Guardian child sessions. When a verified parent thread ID can be located in the local Codex session store, **Open Parent Session** opens that rollout through the normal session workflow; otherwise the app reports that the parent was not found locally.

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

Each human-authored `[YOU]` block starts a work unit. Following Codex/assistant responses, tool calls, tool results, and report messages remain in that unit until the next `[YOU]` block. Recognized delegated handoff tasks and ambient UI context do not become work-unit requests. Referenced-conversation title, conversation ID, and preview-availability status are recorded in `000_index.md`, but cached preview messages are not exported. `manifest.json` records the generated bundle and supports safe, compatible refreshes.

Export always uses the full source session, not the currently filtered transcript view. Re-exporting the same compatible session bundle refreshes its generated files through `manifest.json`. After a successful export, the app opens the generated bundle folder in the operating system's file explorer.

Directory and file names use local, user-facing time. Original timestamps remain in generated content and metadata where they are available.

> Exported worklogs may contain prompts, local paths, command output, code snippets, and project-specific details. Review a bundle before sharing it.

## Build and run on Windows

To build from a source checkout on Windows, install Node.js with npm and the Rust stable toolchain. From the repository root, double-click `build-and-run.bat` or run:

```text
build-and-run.bat
```

The script builds the release application without generating an installer, creates the following portable archive, and then starts the application:

```text
release\Codex-JSONL-Observatory_1.1.1_windows-x64-portable.zip
```

The archive contains `codex-jsonl-observatory.exe`, `LICENSE`, and a bilingual `README.txt`. The built application is started from:

```text
frontend\src-tauri\target\release\codex-jsonl-observatory.exe
```

## Runtime and development

The application uses a Svelte frontend in a Tauri desktop shell. Tauri calls the Rust parser and export boundary directly for local JSONL processing.

Development and verification commands run from `frontend/`:

```text
npm run check
npm run build
npm run tauri:dev
npm run tauri:build
```
