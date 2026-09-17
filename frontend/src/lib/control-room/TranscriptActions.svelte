<script lang="ts">
  import type { LoadStatus } from '../load-workflow'

  interface Props {
    status: LoadStatus
    hasSelectedPath: boolean
    canCaptureTranscript: boolean
    actionStatusMessage: string
    onCapture: () => void | Promise<void>
    onTop: () => void
    onRefresh: () => void | Promise<void>
  }

  let {
    status,
    hasSelectedPath,
    canCaptureTranscript,
    actionStatusMessage,
    onCapture,
    onTop,
    onRefresh,
  }: Props = $props()
</script>

<footer class="transcript-footer" aria-label="Transcript actions">
  <span class="transcript-action-status" aria-live="polite">
    {actionStatusMessage}
  </span>
  <div class="transcript-actions">
    <button
      type="button"
      class="compact-action-button"
      disabled={status !== 'loaded' || !canCaptureTranscript}
      onclick={onCapture}
    >
      Capture Transcript
    </button>
    <button
      type="button"
      class="compact-action-button"
      disabled={status !== 'loaded'}
      onclick={onTop}
    >
      Top
    </button>
    <button
      type="button"
      class="compact-action-button"
      disabled={!hasSelectedPath}
      onclick={() => onRefresh()}
    >
      Refresh
    </button>
  </div>
</footer>
