<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener'
  import type { ObservedEventCountDto, ReferencedConversationDto, TranscriptBlockDto } from '../parse-contract'
  import { renderLabelForKind } from './render-labels'
  import type { TranscriptThemeName } from './transcript-themes'
  import ConversationReferences from './ConversationReferences.svelte'

  const COSMIC_HORIZON_URL = 'https://riu-salze-studio.gitbook.io/cosmic-horizon'

  interface Props {
    theme: TranscriptThemeName
    isLoaded: boolean
    showIdentityNote: boolean
    observedEventCounts: ObservedEventCountDto[]
    blocks: TranscriptBlockDto[]
    references: ReferencedConversationDto[]
  }

  let { theme, isLoaded, showIdentityNote, observedEventCounts, blocks, references }: Props = $props()
  let collapsedBlocks = $state<Record<number, boolean>>({})

  const separator = '========================================================================'
  const displayedEventCounts = $derived(
    [...observedEventCounts].sort((left, right) => right.count - left.count).slice(0, 8),
  )

  function toggleBlock(index: number) {
    collapsedBlocks[index] = !collapsedBlocks[index]
  }

  function displayLabel(block: TranscriptBlockDto) {
    const sourceLabel = block.label.trim()

    if (sourceLabel !== '') {
      return sourceLabel
    }

    return `[${renderLabelForKind(block.entry_type, '').label}]`
  }

  async function visitCosmicHorizon(event: MouseEvent) {
    event.preventDefault()
    await openUrl(COSMIC_HORIZON_URL)
  }
</script>

<div class="terminal-transcript" aria-label="Terminal transcript">
  <div class="terminal-title">Codex JSONL Observatory</div>
  <div class="terminal-blank" aria-hidden="true"></div>

  {#if !isLoaded}
    <div>Ready.</div>
    <div class="terminal-blank" aria-hidden="true"></div>
    {#if showIdentityNote}
      <div>Codex JSONL Observatory is built from the Cosmic Horizon approach to observable AI-assisted work.</div>
      <div>Cosmic Horizon Archive&nbsp; <a class="terminal-visit-link" href={COSMIC_HORIZON_URL} onclick={visitCosmicHorizon}>[Visit]</a></div>
      <div class="terminal-blank" aria-hidden="true"></div>
      <div>Select a local JSONL session to begin.</div>
    {/if}
    <div class="terminal-metadata">Current theme: {theme}</div>
  {:else}
    <div class="terminal-separator">{separator}</div>
    <div class="terminal-blank" aria-hidden="true"></div>
    {#if references.length > 0}
      <ConversationReferences {references} />
      <div class="terminal-blank" aria-hidden="true"></div>
    {/if}

    {#if blocks.length === 0}
      <div>No renderable chat messages found in this JSONL file.</div>

      {#if displayedEventCounts.length > 0}
        <div class="terminal-blank" aria-hidden="true"></div>
        <div class="terminal-metadata">Observed event types:</div>
        {#each displayedEventCounts as eventCount}
          <div class="terminal-metadata">- {eventCount.event}: {eventCount.count}</div>
        {/each}
      {/if}

      <div class="terminal-blank" aria-hidden="true"></div>
    {:else}
      {#each blocks as block, index}
        {@const label = renderLabelForKind(block.entry_type, block.label)}
        {@const isCollapsed = collapsedBlocks[index] ?? false}
        <section class="terminal-block" data-family={label.family} data-kind={block.entry_type}>
          <button
            type="button"
            class="terminal-block-toggle"
            aria-expanded={!isCollapsed}
            onclick={() => toggleBlock(index)}
          >
            {isCollapsed ? '[>]' : '[v]'} {displayLabel(block)}
          </button>
          {#if !isCollapsed}
            <pre class="terminal-block-content">{block.content}</pre>
          {/if}
        </section>

        {#if index !== blocks.length - 1}
          <div class="terminal-blank" aria-hidden="true"></div>
        {/if}
      {/each}

      <div class="terminal-blank" aria-hidden="true"></div>
      <div class="terminal-blank" aria-hidden="true"></div>
    {/if}

    <div class="terminal-separator">{separator}</div>
  {/if}
</div>
