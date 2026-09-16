<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener'
  import type { ObservedEventCountDto, TranscriptBlockDto } from '../parse-contract'
  import { renderLabelForKind } from './render-labels'
  import type { TranscriptThemeName } from './transcript-themes'

  const COSMIC_HORIZON_URL = 'https://riu-salze-studio.gitbook.io/cosmic-horizon'

  interface Props {
    theme: TranscriptThemeName
    isLoaded: boolean
    observedEventCounts: ObservedEventCountDto[]
    blocks: TranscriptBlockDto[]
  }

  let { theme, isLoaded, observedEventCounts, blocks }: Props = $props()
  let collapsedBlocks = $state<Record<number, boolean>>({})

  const displayedEventCounts = $derived(
    [...observedEventCounts].sort((left, right) => right.count - left.count).slice(0, 8),
  )

  function toggleBlock(index: number) {
    collapsedBlocks[index] = !collapsedBlocks[index]
  }

  async function visitCosmicHorizon(event: MouseEvent) {
    event.preventDefault()
    await openUrl(COSMIC_HORIZON_URL)
  }
</script>

<div class="markdown-transcript" aria-label="Markdown transcript">
  <header class="markdown-document-header">
    <p class="markdown-kicker">Markdown Style</p>
    <h2>Codex JSONL Observatory</h2>
    {#if !isLoaded}
      <div class="markdown-empty-state">
        <p>Ready.</p>
        <p>Codex JSONL Observatory is built from the Cosmic Horizon approach to observable AI-assisted work.</p>
        <p>
          Cosmic Horizon Archive
          <a href={COSMIC_HORIZON_URL} onclick={visitCosmicHorizon}>[Visit]</a>
        </p>
        <p>Select a local JSONL session to begin.</p>
        <p class="markdown-empty-metadata">Current theme: {theme}</p>
      </div>
    {/if}
  </header>

  {#if isLoaded}
    {#if blocks.length === 0}
      <section class="markdown-notice">
        <p>No renderable chat messages found in this JSONL file.</p>
        {#if displayedEventCounts.length > 0}
          <h3>Observed event types</h3>
          <ul>
            {#each displayedEventCounts as eventCount}
              <li>{eventCount.event}: {eventCount.count}</li>
            {/each}
          </ul>
        {/if}
      </section>
    {:else}
      <div class="markdown-blocks">
        {#each blocks as block, index}
          {@const label = renderLabelForKind(block.entry_type, block.label)}
          {@const isCollapsed = collapsedBlocks[index] ?? false}
          <section class="markdown-block" data-family={label.family} data-kind={block.entry_type}>
            <button
              type="button"
              class="markdown-block-toggle"
              aria-expanded={!isCollapsed}
              onclick={() => toggleBlock(index)}
            >
              <span class="transcript-toggle-marker" aria-hidden="true">{isCollapsed ? '>' : 'v'}</span>
              <strong>[{label.label}]</strong>
            </button>
            {#if !isCollapsed}
              <pre>{block.content}</pre>
            {/if}
          </section>
        {/each}
      </div>
    {/if}
  {/if}
</div>
