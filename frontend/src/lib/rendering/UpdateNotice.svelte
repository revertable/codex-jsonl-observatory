<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { RELEASES_PAGE_URL, type AvailableUpdate } from '../update-check'

  interface Props {
    update: AvailableUpdate
  }

  let { update }: Props = $props()

  async function visitReleases(event: MouseEvent) {
    event.preventDefault()
    await openUrl(RELEASES_PAGE_URL)
  }
</script>

<span class="update-notice">
  New version available: v{update.latestVersion}.
  <a href={RELEASES_PAGE_URL} onclick={visitReleases}>[View GitHub Releases]</a>
</span>
