'use strict'

function isEscaped(text, index) {
  let backslashes = 0
  for (let cursor = index - 1; cursor >= 0 && text[cursor] === '\\'; cursor -= 1) {
    backslashes += 1
  }
  return backslashes % 2 === 1
}

function findBalancedEnd(text, start, open, close) {
  let depth = 0
  for (let index = start; index < text.length; index += 1) {
    if (text[index] === '\\') {
      index += 1
      continue
    }
    if (text[index] === open) {
      depth += 1
    } else if (text[index] === close) {
      depth -= 1
      if (depth === 0) return index
    }
  }
  return -1
}

function findFirstUnsafeBracket(text) {
  for (let index = 0; index < text.length; index += 1) {
    if (text[index] !== '[' || isEscaped(text, index)) continue

    const labelEnd = findBalancedEnd(text, index, '[', ']')
    const opener =
      index > 0 && text[index - 1] === '!' && !isEscaped(text, index - 1)
        ? index - 1
        : index
    if (labelEnd === -1 || labelEnd === text.length - 1) return opener

    if (text[labelEnd + 1] !== '(') {
      index = labelEnd
      continue
    }

    const destinationEnd = findBalancedEnd(text, labelEnd + 1, '(', ')')
    if (destinationEnd === -1) return opener
    index = destinationEnd
  }
  return -1
}

function findSafeBoundary(text) {
  const firstUnclosedBracket = findFirstUnsafeBracket(text)

  let firstUnclosedLt = -1
  for (let index = 0; index < text.length; index += 1) {
    if (text[index] === '<') {
      const nextGt = text.indexOf('>', index)
      const nextLt = text.indexOf('<', index + 1)
      if (nextGt >= 0 && (nextLt === -1 || nextGt < nextLt)) {
        index = nextGt
      } else {
        firstUnclosedLt = index
        break
      }
    }
  }

  let boundary = text.length
  if (firstUnclosedBracket >= 0) boundary = Math.min(boundary, firstUnclosedBracket)
  if (firstUnclosedLt >= 0) boundary = Math.min(boundary, firstUnclosedLt)
  if (boundary < text.length) return boundary

  if (text.endsWith('\\')) return text.length - 1

  const trailingBackticks = text.match(/`+$/)
  if (trailingBackticks) {
    const totalBackticks = (text.match(/`/g) ?? []).length
    if (totalBackticks % 2 !== 0) return text.length - trailingBackticks[0].length
  }

  return text.length
}

function createMarkdownStreamBuffer(options = {}) {
  if (options === null || typeof options !== 'object' || Array.isArray(options)) {
    throw new TypeError('options must be an object')
  }

  const { maxHoldMs = 50, now = Date.now } = options
  if (!Number.isFinite(maxHoldMs) || maxHoldMs < 0) {
    throw new RangeError('maxHoldMs must be a finite nonnegative number')
  }
  if (typeof now !== 'function') {
    throw new TypeError('now must be a function')
  }

  let pending = ''
  let heldSince = null

  return {
    push(text) {
      if (typeof text !== 'string') {
        throw new TypeError('text must be a string')
      }

      pending += text
      const boundary = findSafeBoundary(pending)
      if (boundary === 0) {
        if (heldSince === null) heldSince = now()
        if (now() - heldSince >= maxHoldMs) {
          const output = pending
          pending = ''
          heldSince = null
          return [output]
        }
        return []
      }

      const output = pending.slice(0, boundary)
      pending = pending.slice(boundary)
      heldSince = pending.length > 0 ? (heldSince ?? now()) : null
      return output ? [output] : []
    },
    flush() {
      const output = pending
      pending = ''
      heldSince = null
      return output
    },
  }
}

exports.createMarkdownStreamBuffer = createMarkdownStreamBuffer
