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
  import SessionHeader from './lib/control-room/SessionHeader.svelte'
  import TranscriptActions from './lib/control-room/TranscriptActions.svelte'
  import ChatTranscript from './lib/rendering/ChatTranscript.svelte'
  import MarkdownTranscript from './lib/rendering/MarkdownTranscript.svelte'
  import TerminalTranscript from './lib/rendering/TerminalTranscript.svelte'
  import {
    renderPathForTheme,
    type TranscriptThemeName,
  } from './lib/rendering/transcript-themes'
  import type { ApiErrorDto } from './lib/parse-contract'

  let workflow: LoadWorkflowState = createInitialLoadWorkflowState()
  let actionStatusMessage = ''
  let transcriptActionStatusMessage = ''
  let isExportingWorklog = false
  let transcriptElement: HTMLElement | null = null
  let selectedTheme: TranscriptThemeName = 'Terminal Style'

  function handlePathChange(path: string) {
    workflow = selectPath(workflow, path)
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

  function handleFilterChange(key: keyof LoadWorkflowState['filter'], value: boolean) {
    workflow = updateFilter(workflow, key, value)
    transcriptActionStatusMessage = ''
  }

  function handleThemeChange(theme: TranscriptThemeName) {
    selectedTheme = theme
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
  <SessionHeader
    status={workflow.status}
    selectedPath={workflow.selected_file.path}
    friendlySelectedPath={workflow.selected_file.path
      ? displayFriendlyPath(workflow.selected_file.path)
      : 'No JSONL path selected.'}
    metadata={workflow.loaded_file.metadata}
    filter={workflow.filter}
    {selectedTheme}
    {isExportingWorklog}
    {actionStatusMessage}
    errorMessage={workflow.error?.message ?? null}
    onChooseJsonl={chooseJsonlPath}
    onPathChange={handlePathChange}
    onRefresh={loadSelectedJsonl}
    onReset={resetToIdle}
    onCopyResume={copyResumeCommand}
    onExportWorklog={handleExportWorklog}
    onFilterChange={handleFilterChange}
    onThemeChange={handleThemeChange}
  />

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

  <TranscriptActions
    status={workflow.status}
    hasSelectedPath={hasSelectedPath()}
    actionStatusMessage={transcriptActionStatusMessage}
    onCapture={captureTranscript}
    onRefresh={loadSelectedJsonl}
  />
</main>
