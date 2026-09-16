<script lang="ts">
  import type { LoadStatus } from '../load-workflow'
  import type { FilterDto, LoadedFileMetadataDto } from '../parse-contract'
  import { transcriptThemes, type TranscriptThemeName } from '../rendering/transcript-themes'

  interface Props {
    status: LoadStatus
    selectedPath: string
    friendlySelectedPath: string
    metadata: LoadedFileMetadataDto | null
    filter: FilterDto
    selectedTheme: TranscriptThemeName
    isExportingWorklog: boolean
    actionStatusMessage: string
    errorMessage: string | null
    onChooseJsonl: () => void | Promise<void>
    onPathChange: (path: string) => void
    onRefresh: () => void | Promise<void>
    onReset: () => void
    onCopyResume: () => void | Promise<void>
    onExportWorklog: () => void | Promise<void>
    onFilterChange: (key: keyof FilterDto, value: boolean) => void
    onThemeChange: (theme: TranscriptThemeName) => void
  }

  let {
    status,
    selectedPath,
    friendlySelectedPath,
    metadata,
    filter,
    selectedTheme,
    isExportingWorklog,
    actionStatusMessage,
    errorMessage,
    onChooseJsonl,
    onPathChange,
    onRefresh,
    onReset,
    onCopyResume,
    onExportWorklog,
    onFilterChange,
    onThemeChange,
  }: Props = $props()

  const filterOptions = [
    ['show_you', 'You'],
    ['show_codex', 'Codex'],
    ['show_tool_call', 'Tool calls'],
    ['show_tool_result', 'Tool results'],
    ['show_meta', 'Meta'],
  ] as const
</script>

<header class="app-header" aria-labelledby="app-title">
  <div class="toolbar">
    <div class="product-heading">
      <p class="eyebrow">Local transcript viewer</p>
      <h1 id="app-title">Codex JSONL Observatory</h1>
    </div>

    <div class="toolbar-actions">
      <button type="button" onclick={onChooseJsonl}>Select JSONL</button>

      <label class="manual-path-field">
        <span>Manual path (use Refresh to load)</span>
        <input
          type="text"
          value={selectedPath}
          placeholder="Paste a local JSONL path"
          oninput={(event) => onPathChange(event.currentTarget.value)}
        />
      </label>

      <button
        type="button"
        class="refresh-button"
        disabled={selectedPath.trim() === ''}
        onclick={() => onRefresh()}
      >
        Refresh
      </button>
      {#if status === 'loaded'}
        <button
          type="button"
          class="status status-reset"
          data-status={status}
          aria-label="Reset loaded session"
          title="Reset to idle"
          disabled={isExportingWorklog}
          onclick={onReset}
        >
          {status}
        </button>
      {:else}
        <span class="status" data-status={status}>{status}</span>
      {/if}
    </div>
  </div>

  <section class="selection-panel" aria-label="Current selection">
    <div class="selected-path-row">
      <span>Selected path</span>
      <strong title={friendlySelectedPath}>{friendlySelectedPath}</strong>
    </div>

    <dl class="selection-metadata">
      <div>
        <dt>File</dt>
        <dd>{metadata?.file_name ?? 'Not loaded'}</dd>
      </div>
      <div>
        <dt>Session ID</dt>
        <dd>{metadata?.session_id ?? 'Not detected'}</dd>
      </div>
    </dl>

    <div class="resume-row">
      <span>Resume command</span>
      <code>{metadata?.resume_command ?? 'Not available'}</code>
      <div class="resume-actions">
        <button
          type="button"
          class="secondary resume-action-button"
          disabled={metadata?.resume_command == null}
          onclick={onCopyResume}
        >
          Copy Resume Command
        </button>
        <button
          type="button"
          class="secondary resume-action-button"
          disabled={status !== 'loaded' || isExportingWorklog}
          onclick={onExportWorklog}
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
            checked={filter[key]}
            onchange={(event) => onFilterChange(key, event.currentTarget.checked)}
          />
          <span>{label}</span>
        </label>
      {/each}
    </div>
    <label class="theme-selector">
      <span>Theme</span>
      <select
        value={selectedTheme}
        onchange={(event) => onThemeChange(event.currentTarget.value as TranscriptThemeName)}
      >
        {#each transcriptThemes as theme}
          <option value={theme}>{theme}</option>
        {/each}
      </select>
    </label>
  </section>

  {#if status === 'loading'}
    <p class="status-message" aria-live="polite">Loading the selected JSONL file…</p>
  {:else if status === 'error'}
    <p class="status-message error" aria-live="assertive">
      {errorMessage ?? 'The selected JSONL file could not be loaded.'}
    </p>
  {/if}
</header>
