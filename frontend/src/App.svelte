<script lang="ts">
  import {
    applyParseResponse,
    beginLoad,
    createInitialLoadWorkflowState,
    defaultFilterState,
    failLoad,
    selectPath,
    updateFilter,
    type LoadWorkflowState,
  } from './lib/load-workflow'
  import {
    exportWorklog,
    parseSelectedJsonl,
    selectJsonlPath,
    selectWorklogParentDirectory,
  } from './lib/tauri-bridge'
  import ChatTranscript from './lib/rendering/ChatTranscript.svelte'
  import MarkdownTranscript from './lib/rendering/MarkdownTranscript.svelte'
  import TerminalTranscript from './lib/rendering/TerminalTranscript.svelte'
  import {
    renderPathForTheme,
    transcriptThemes,
    type TranscriptThemeName,
  } from './lib/rendering/transcript-themes'
  import type { ApiErrorDto } from './lib/parse-contract'

  let workflow: LoadWorkflowState = createInitialLoadWorkflowState()
  let actionStatusMessage = ''
  let transcriptActionStatusMessage = ''
  let isExportingWorklog = false
  let transcriptElement: HTMLElement | null = null
  let selectedTheme: TranscriptThemeName = 'Terminal Style'

  const filterOptions = [
    ['show_you', 'You'],
    ['show_codex', 'Codex'],
    ['show_tool_call', 'Tool calls'],
    ['show_tool_result', 'Tool results'],
    ['show_meta', 'Meta'],
  ] as const

  function handlePathInput(event: Event) {
    const target = event.currentTarget as HTMLInputElement
    workflow = selectPath(workflow, target.value)
    actionStatusMessage = ''
    transcriptActionStatusMessage = ''
  }

  async function chooseJsonlPath() {
    const selectedPath = await selectJsonlPath()

    if (selectedPath !== null) {
      workflow = selectPath(workflow, selectedPath)
      actionStatusMessage = ''
      transcriptActionStatusMessage = ''
      await loadSelectedJsonl(selectedPath)
    }
  }

  async function loadSelectedJsonl(pathOverride?: string) {
    const path = (pathOverride ?? workflow.selected_file.path).trim()

    if (path === '') {
      return
    }

    workflow = beginLoad(workflow)
    transcriptActionStatusMessage = ''

    try {
      const response = await parseSelectedJsonl(path, defaultFilterState)
      workflow = applyParseResponse(workflow, response)
      actionStatusMessage = ''
    } catch (error) {
      workflow = failLoad(workflow, normalizeLoadError(error))
      actionStatusMessage = ''
    }
  }

  function handleFilterInput(key: keyof LoadWorkflowState['filter'], event: Event) {
    const target = event.currentTarget as HTMLInputElement
    workflow = updateFilter(workflow, key, target.checked)
    transcriptActionStatusMessage = ''
  }

  function handleThemeInput(event: Event) {
    const target = event.currentTarget as HTMLSelectElement
    selectedTheme = target.value as TranscriptThemeName
    transcriptActionStatusMessage = ''
  }

  function resetToIdle() {
    if (isExportingWorklog) {
      return
    }

    workflow = createInitialLoadWorkflowState()
    actionStatusMessage = ''
    transcriptActionStatusMessage = ''
    selectedTheme = 'Terminal Style'
  }

  function displayFriendlyPath(path: string) {
    return path.replace(/^\\\\\?\\UNC\\/i, '\\\\').replace(/^\\\\\?\\/, '')
  }

  function transcriptKey() {
    const filter = workflow.filter
    return [
      workflow.loaded_file.metadata?.absolute_path ?? 'unloaded',
      filter.show_you,
      filter.show_codex,
      filter.show_tool_call,
      filter.show_tool_result,
      filter.show_meta,
      selectedTheme,
    ].join('|')
  }

  async function copyResumeCommand() {
    const command = workflow.loaded_file.metadata?.resume_command

    if (command == null || command.trim() === '') {
      return
    }

    try {
      if (navigator.clipboard == null) {
        throw new Error('Clipboard access is unavailable.')
      }

      await navigator.clipboard.writeText(command)
      actionStatusMessage = 'Copied.'
    } catch {
      actionStatusMessage = 'Copy failed.'
    }
  }

  async function captureTranscript() {
    if (workflow.status !== 'loaded' || transcriptElement === null) {
      return
    }

    const transcriptText = transcriptElement.innerText.trim()

    if (transcriptText === '') {
      transcriptActionStatusMessage = 'No transcript text available.'
      return
    }

    try {
      if (navigator.clipboard == null) {
        throw new Error('Clipboard access is unavailable.')
      }

      await navigator.clipboard.writeText(transcriptText)
      transcriptActionStatusMessage = 'Transcript captured.'
    } catch {
      transcriptActionStatusMessage = 'Capture failed.'
    }
  }

  async function handleExportWorklog() {
    const metadata = workflow.loaded_file.metadata

    if (workflow.status !== 'loaded' || metadata === null || isExportingWorklog) {
      return
    }

    isExportingWorklog = true

    try {
      const parentDirectory = await selectWorklogParentDirectory(metadata.absolute_path)
      if (parentDirectory === null) {
        actionStatusMessage = 'Export cancelled.'
        return
      }

      actionStatusMessage = 'Exporting worklog…'
      const result = await exportWorklog(metadata.absolute_path, parentDirectory)
      if (!result.folder_opened) {
        actionStatusMessage = result.refreshed
          ? 'Worklog refreshed, but folder could not be opened.'
          : 'Worklog exported, but folder could not be opened.'
      } else {
        actionStatusMessage = result.refreshed
          ? `Worklog refreshed: ${displayFriendlyPath(result.bundle_path)}`
          : `Worklog exported: ${displayFriendlyPath(result.bundle_path)}`
      }
    } catch (error) {
      const apiError = normalizeLoadError(error)
      actionStatusMessage =
        apiError.code === 'target_not_safe_to_overwrite'
          ? 'Target not safe to overwrite.'
          : `Worklog export failed: ${apiError.message}`
    } finally {
      isExportingWorklog = false
    }
  }

  function hasSelectedPath() {
    return workflow.selected_file.path.trim() !== ''
  }

  function normalizeLoadError(error: unknown): ApiErrorDto {
    if (isApiError(error)) {
      return error
    }

    if (error instanceof Error) {
      return { code: 'tauri_command_failed', message: error.message }
    }

    return { code: 'tauri_command_failed', message: String(error) }
  }

  function isApiError(error: unknown): error is ApiErrorDto {
    return (
      typeof error === 'object' &&
      error !== null &&
      'code' in error &&
      'message' in error &&
      typeof (error as ApiErrorDto).code === 'string' &&
      typeof (error as ApiErrorDto).message === 'string'
    )
  }
</script>

<main class="control-room">
  <header class="app-header" aria-labelledby="app-title">
    <div class="toolbar">
      <div class="product-heading">
        <p class="eyebrow">Local transcript viewer</p>
        <h1 id="app-title">Codex JSONL Observatory</h1>
      </div>

      <div class="toolbar-actions">
        <button type="button" onclick={chooseJsonlPath}>Select JSONL</button>

        <label class="manual-path-field">
          <span>Manual path (use Refresh to load)</span>
          <input
            type="text"
            value={workflow.selected_file.path}
            placeholder="Paste a local JSONL path"
            oninput={handlePathInput}
          />
        </label>

        <button
          type="button"
          class="refresh-button"
          disabled={!hasSelectedPath()}
          onclick={() => loadSelectedJsonl()}
        >
          Refresh
        </button>
        {#if workflow.status === 'loaded'}
          <button
            type="button"
            class="status status-reset"
            data-status={workflow.status}
            aria-label="Reset loaded session"
            title="Reset to idle"
            disabled={isExportingWorklog}
            onclick={resetToIdle}
          >
            {workflow.status}
          </button>
        {:else}
          <span class="status" data-status={workflow.status}>{workflow.status}</span>
        {/if}
      </div>
    </div>

    <section class="selection-panel" aria-label="Current selection">
      <div class="selected-path-row">
        <span>Selected path</span>
        <strong
          title={workflow.selected_file.path
            ? displayFriendlyPath(workflow.selected_file.path)
            : 'No JSONL path selected.'}
        >
          {workflow.selected_file.path
            ? displayFriendlyPath(workflow.selected_file.path)
            : 'No JSONL path selected.'}
        </strong>
      </div>

      <dl class="selection-metadata">
        <div>
          <dt>File</dt>
          <dd>{workflow.loaded_file.metadata?.file_name ?? 'Not loaded'}</dd>
        </div>
        <div>
          <dt>Session ID</dt>
          <dd>{workflow.loaded_file.metadata?.session_id ?? 'Not detected'}</dd>
        </div>
      </dl>

      <div class="resume-row">
        <span>Resume command</span>
        <code>{workflow.loaded_file.metadata?.resume_command ?? 'Not available'}</code>
        <div class="resume-actions">
          <button
            type="button"
            class="secondary resume-action-button"
            disabled={workflow.loaded_file.metadata?.resume_command == null}
            onclick={copyResumeCommand}
          >
            Copy Resume Command
          </button>
          <button
            type="button"
            class="secondary resume-action-button"
            disabled={workflow.status !== 'loaded' || isExportingWorklog}
            onclick={handleExportWorklog}
          >
            Export Worklog
          </button>
        </div>
        <span class="action-status" aria-live="polite">{actionStatusMessage}</span>
      </div>
    </section>

    <section class="filter-bar" aria-label="Transcript filters">
      <h2>Filters</h2>
      <div class="filter-list">
        {#each filterOptions as [key, label]}
          <label>
            <input
              type="checkbox"
              checked={workflow.filter[key]}
              onchange={(event) => handleFilterInput(key, event)}
            />
            <span>{label}</span>
          </label>
        {/each}
      </div>
      <label class="theme-selector">
        <span>Theme</span>
        <select value={selectedTheme} onchange={handleThemeInput}>
          {#each transcriptThemes as theme}
            <option value={theme}>{theme}</option>
          {/each}
        </select>
      </label>
    </section>

    {#if workflow.status === 'loading'}
      <p class="status-message" aria-live="polite">Loading the selected JSONL file…</p>
    {:else if workflow.status === 'error'}
      <p class="status-message error" aria-live="assertive">
        {workflow.error?.message ?? 'The selected JSONL file could not be loaded.'}
      </p>
    {/if}
  </header>

  <section class="terminal-section" aria-label="Transcript view">
    <article
      bind:this={transcriptElement}
      class:terminal-panel={renderPathForTheme(selectedTheme) === 'terminal'}
      class:theme-panel={renderPathForTheme(selectedTheme) !== 'terminal'}
    >
      {#key transcriptKey()}
        {#if renderPathForTheme(selectedTheme) === 'terminal'}
          <TerminalTranscript
            theme={selectedTheme}
            isLoaded={workflow.status === 'loaded'}
            showIdentityNote={workflow.status !== 'loaded'}
            observedEventCounts={workflow.loaded_file.observed_event_counts}
            blocks={workflow.observations.transcript_blocks}
          />
        {:else if renderPathForTheme(selectedTheme) === 'markdown'}
          <MarkdownTranscript
            theme={selectedTheme}
            isLoaded={workflow.status === 'loaded'}
            observedEventCounts={workflow.loaded_file.observed_event_counts}
            blocks={workflow.observations.transcript_blocks}
          />
        {:else}
          <ChatTranscript
            theme={selectedTheme as 'DM Style' | 'DM Style (Dark)'}
            isLoaded={workflow.status === 'loaded'}
            observedEventCounts={workflow.loaded_file.observed_event_counts}
            blocks={workflow.observations.transcript_blocks}
          />
        {/if}
      {/key}
    </article>
  </section>

  <footer class="transcript-footer" aria-label="Transcript actions">
    <span class="transcript-action-status" aria-live="polite">
      {transcriptActionStatusMessage}
    </span>
    <div class="transcript-actions">
      <button
        type="button"
        class="refresh-button"
        disabled={workflow.status !== 'loaded'}
        onclick={captureTranscript}
      >
        Capture Transcript
      </button>
      <button
        type="button"
        class="refresh-button"
        disabled={!hasSelectedPath()}
        onclick={() => loadSelectedJsonl()}
      >
        Refresh
      </button>
    </div>
  </footer>
</main>
