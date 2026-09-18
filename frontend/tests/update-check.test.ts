import assert from 'node:assert/strict'
import test from 'node:test'

import {
  RELEASES_API_URL,
  findAvailableUpdate,
  type UpdateFetch,
} from '../src/lib/update-check.ts'

function releaseResponse(tagName: unknown, ok = true): UpdateFetch {
  return async () => ({
    ok,
    json: async () => ({ tag_name: tagName }),
  })
}

test('reports only a stable public version newer than the running app', async () => {
  assert.deepEqual(
    await findAvailableUpdate('1.1.3', releaseResponse('v1.2.0')),
    {
      currentVersion: '1.1.3',
      latestVersion: '1.2.0',
    },
  )
  assert.equal(await findAvailableUpdate('1.1.3', releaseResponse('v1.1.3')), null)
  assert.equal(await findAvailableUpdate('1.1.3', releaseResponse('v1.1.2')), null)
})

test('compares major minor and patch components numerically', async () => {
  assert.deepEqual(
    await findAvailableUpdate('v1.9.9', releaseResponse('v1.10.0')),
    {
      currentVersion: '1.9.9',
      latestVersion: '1.10.0',
    },
  )
  assert.deepEqual(
    await findAvailableUpdate('1.9.9', releaseResponse('2.0.0')),
    {
      currentVersion: '1.9.9',
      latestVersion: '2.0.0',
    },
  )
})

test('uses the fixed GitHub endpoint with the recommended media type', async () => {
  let requestedUrl: string | undefined
  let requestedInit: RequestInit | undefined
  const fetchRelease: UpdateFetch = async (url, init) => {
    requestedUrl = url
    requestedInit = init
    return {
      ok: true,
      json: async () => ({ tag_name: 'v1.1.4' }),
    }
  }

  await findAvailableUpdate('1.1.3', fetchRelease)

  assert.equal(requestedUrl, RELEASES_API_URL)
  assert.deepEqual(requestedInit?.headers, {
    Accept: 'application/vnd.github+json',
  })
  assert.ok(requestedInit?.signal instanceof AbortSignal)
})

test('ignores malformed versions payloads failures and non-success responses', async () => {
  assert.equal(await findAvailableUpdate('development', releaseResponse('v1.2.0')), null)
  assert.equal(await findAvailableUpdate('1.1.3', releaseResponse('v1.2')), null)
  assert.equal(await findAvailableUpdate('1.1.3', releaseResponse('v1.2.0-beta.1')), null)
  assert.equal(await findAvailableUpdate('1.1.3', releaseResponse(null)), null)
  assert.equal(await findAvailableUpdate('1.1.3', releaseResponse('v1.2.0', false)), null)
  assert.equal(
    await findAvailableUpdate('1.1.3', async () => {
      throw new Error('offline')
    }),
    null,
  )
})

test('aborts a stalled update request without surfacing an error', async () => {
  const stalledFetch: UpdateFetch = async (_url, init) =>
    new Promise((_resolve, reject) => {
      init.signal?.addEventListener('abort', () => reject(new Error('aborted')))
    })

  assert.equal(await findAvailableUpdate('1.1.3', stalledFetch, 1), null)
})
