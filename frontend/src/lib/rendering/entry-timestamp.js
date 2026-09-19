/**
 * @typedef {object} FormattedEntryTimestamp
 * @property {string} datetime
 * @property {string} label
 */

const RFC3339_TIMESTAMP =
  /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/

/**
 * @param {string | null} timestamp
 * @returns {FormattedEntryTimestamp | null}
 */
export function formatEntryTimestamp(timestamp) {
  const source = timestamp?.trim()
  if (!source || !RFC3339_TIMESTAMP.test(source)) {
    return null
  }

  const instant = new Date(source)
  if (Number.isNaN(instant.getTime())) {
    return null
  }

  return {
    datetime: source,
    label: `${instant.getFullYear()}-${twoDigits(instant.getMonth() + 1)}-${twoDigits(instant.getDate())} ${twoDigits(instant.getHours())}:${twoDigits(instant.getMinutes())}:${twoDigits(instant.getSeconds())}`,
  }
}

/** @param {number} value */
function twoDigits(value) {
  return value.toString().padStart(2, '0')
}
