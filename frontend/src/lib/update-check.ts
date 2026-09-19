export const RELEASES_API_URL =
  'https://api.github.com/repos/revertable/codex-session-observatory/releases/latest'
export const RELEASES_PAGE_URL =
  'https://github.com/revertable/codex-session-observatory/releases/latest'

const DEFAULT_TIMEOUT_MS = 5_000

export type PublicReleaseStatus =
  | {
      status: 'update-available'
      currentVersion: string
      latestVersion: string
    }
  | {
      status: 'current'
      currentVersion: string
      latestVersion: string
    }
  | {
      status: 'development'
      currentVersion: string
      latestVersion: string
    }

interface ReleaseVersion {
  currentVersion: string
  latestVersion: string
}

interface ReleaseResponse {
  ok: boolean
  json: () => Promise<unknown>
}

export type UpdateFetch = (
  input: string,
  init: RequestInit,
) => Promise<ReleaseResponse>

export async function checkPublicReleaseStatus(
  currentVersion: string,
  fetchRelease: UpdateFetch = globalThis.fetch,
  timeoutMs = DEFAULT_TIMEOUT_MS,
): Promise<PublicReleaseStatus | null> {
  const normalizedCurrent = parseStableVersion(currentVersion)
  if (normalizedCurrent === null) {
    return null
  }

  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), timeoutMs)

  try {
    const response = await fetchRelease(RELEASES_API_URL, {
      headers: {
        Accept: 'application/vnd.github+json',
      },
      signal: controller.signal,
    })
    if (!response.ok) {
      return null
    }

    const payload: unknown = await response.json()
    const tagName = releaseTagName(payload)
    const normalizedLatest = tagName === null ? null : parseStableVersion(tagName)
    if (normalizedLatest === null) {
      return null
    }

    const versions: ReleaseVersion = {
      currentVersion: normalizedCurrent.version,
      latestVersion: normalizedLatest.version,
    }
    const comparison = compareVersions(normalizedLatest.parts, normalizedCurrent.parts)

    if (comparison > 0) {
      return { status: 'update-available', ...versions }
    }

    if (comparison === 0) {
      return { status: 'current', ...versions }
    }

    return { status: 'development', ...versions }
  } catch {
    return null
  } finally {
    clearTimeout(timeout)
  }
}

interface StableVersion {
  version: string
  parts: readonly [number, number, number]
}

function parseStableVersion(value: string): StableVersion | null {
  const match = /^v?(\d+)\.(\d+)\.(\d+)$/.exec(value.trim())
  if (match === null) {
    return null
  }

  const parts = match.slice(1).map(Number) as [number, number, number]
  if (!parts.every(Number.isSafeInteger)) {
    return null
  }

  return {
    version: parts.join('.'),
    parts,
  }
}

function compareVersions(
  left: readonly [number, number, number],
  right: readonly [number, number, number],
): number {
  for (let index = 0; index < left.length; index += 1) {
    const difference = left[index] - right[index]
    if (difference !== 0) {
      return difference
    }
  }

  return 0
}

function releaseTagName(payload: unknown): string | null {
  if (
    typeof payload !== 'object' ||
    payload === null ||
    !('tag_name' in payload) ||
    typeof payload.tag_name !== 'string'
  ) {
    return null
  }

  return payload.tag_name
}
