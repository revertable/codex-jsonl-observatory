<script lang="ts">
  import type { ReferencedConversationDto } from '../parse-contract'

  interface Props {
    references: ReferencedConversationDto[]
  }

  let { references }: Props = $props()
</script>

{#each references as reference}
  <aside class="conversation-reference" aria-label="Referenced ChatGPT conversation">
    <p class="conversation-reference-kicker">Referenced ChatGPT conversation</p>
    <dl class="conversation-reference-metadata">
      <div>
        <dt>Title</dt>
        <dd>{reference.title ?? 'Not provided'}</dd>
      </div>
      <div>
        <dt>Conversation ID</dt>
        <dd><code>{reference.conversation_id ?? 'Not provided'}</code></dd>
      </div>
    </dl>

  </aside>
{/each}
