<script lang="ts">
  import type { LoadStatus } from '../load-workflow'

  interface Props {
    status: LoadStatus
    hasSelectedPath: boolean
    actionStatusMessage: string
    onCapture: () => void | Promise<void>
    onRefresh: () => void | Promise<void>
  }

  let {
    status,
    hasSelectedPath,
    actionStatusMessage,
    onCapture,
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
      class="refresh-button"
      disabled={status !== 'loaded'}
      onclick={onCapture}
    >
      Capture Transcript
    </button>
    <button
      type="button"
      class="refresh-button"
      disabled={!hasSelectedPath}
      onclick={() => onRefresh()}
    >
      Refresh
    </button>
  </div>
</footer>
