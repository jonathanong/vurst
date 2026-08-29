'use strict'

function isEscaped(text, index) {
  let backslashes = 0
  for (let cursor = index - 1; cursor >= 0 && text[cursor] === '\\'; cursor -= 1) {
    backslashes += 1
  }
  return backslashes % 2 === 1
}

function findBalancedEnd(text, start, open, close, codeSpans) {
  let depth = 0
  let codeSpanIndex = 0
  while (codeSpanIndex < codeSpans.length && codeSpans[codeSpanIndex].end <= start) {
    codeSpanIndex += 1
  }

  for (let index = start; index < text.length; index += 1) {
    const codeSpan = codeSpans[codeSpanIndex]
    if (codeSpan !== undefined && index === codeSpan.start) {
      index = codeSpan.end - 1
      codeSpanIndex += 1
      continue
    }
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

function findFirstUnsafeBracket(text, codeSpans) {
  let searchStart = 0
  let codeSpanIndex = 0
  while (searchStart < text.length) {
    const index = text.indexOf('[', searchStart)
    if (index === -1) return -1

    while (codeSpanIndex < codeSpans.length && codeSpans[codeSpanIndex].end <= index) {
      codeSpanIndex += 1
    }
    const codeSpan = codeSpans[codeSpanIndex]
    if (codeSpan !== undefined && codeSpan.start < index) {
      searchStart = codeSpan.end
      codeSpanIndex += 1
      continue
    }
    if (isEscaped(text, index)) {
      searchStart = index + 1
      continue
    }

    const labelEnd = findBalancedEnd(text, index, '[', ']', codeSpans)
    const opener =
      index > 0 && text[index - 1] === '!' && !isEscaped(text, index - 1)
        ? index - 1
        : index
    if (labelEnd === -1 || labelEnd === text.length - 1) return opener

    if (text[labelEnd + 1] !== '(') {
      searchStart = labelEnd + 1
      continue
    }

    const destinationEnd = findBalancedEnd(text, labelEnd + 1, '(', ')', codeSpans)
    if (destinationEnd === -1) return opener
    searchStart = destinationEnd + 1
  }
  return -1
}

function findBacktickRun(text, searchStart) {
  const start = text.indexOf('`', searchStart)
  if (start === -1) return null

  let end = start + 1
  while (text[end] === '`') end += 1
  return { start, end, length: end - start }
}

function findUnescapedBacktickRun(text, searchStart) {
  let run = findBacktickRun(text, searchStart)
  while (run !== null && isEscaped(text, run.start)) {
    run = findBacktickRun(text, run.end)
  }
  return run
}

function findCodeSpans(text) {
  const complete = []
  let searchStart = 0
  while (searchStart < text.length) {
    const opener = findUnescapedBacktickRun(text, searchStart)
    if (opener === null) return { complete, unclosedStart: -1 }

    let closingSearchStart = opener.end
    while (closingSearchStart < text.length) {
      const closer = findBacktickRun(text, closingSearchStart)
      if (closer === null) return { complete, unclosedStart: opener.start }
      if (closer.length === opener.length) {
        complete.push({ start: opener.start, end: closer.end })
        searchStart = closer.end
        break
      }
      closingSearchStart = closer.end
    }

    if (closingSearchStart >= text.length) return { complete, unclosedStart: opener.start }
  }
  return { complete, unclosedStart: -1 }
}

function findFirstUnclosedLt(text, codeSpans) {
  let codeSpanIndex = 0
  let index = 0
  while (index < text.length) {
    const codeSpan = codeSpans[codeSpanIndex]
    if (codeSpan !== undefined && index === codeSpan.start) {
      index = codeSpan.end
      codeSpanIndex += 1
      continue
    }
    if (text[index] !== '<') {
      index += 1
      continue
    }

    const opener = index
    index += 1
    while (index < text.length) {
      const nestedCodeSpan = codeSpans[codeSpanIndex]
      if (nestedCodeSpan !== undefined && index === nestedCodeSpan.start) {
        index = nestedCodeSpan.end
        codeSpanIndex += 1
        continue
      }
      if (text[index] === '>') break
      if (text[index] === '<') return opener
      index += 1
    }
    if (index === text.length) return opener
    index += 1
  }
  return -1
}

function findSafeBoundary(text) {
  const codeSpans = findCodeSpans(text)
  const firstUnclosedBracket = findFirstUnsafeBracket(text, codeSpans.complete)
  const firstUnclosedLt = findFirstUnclosedLt(text, codeSpans.complete)

  let boundary = text.length
  if (firstUnclosedBracket >= 0) boundary = Math.min(boundary, firstUnclosedBracket)
  if (codeSpans.unclosedStart >= 0) boundary = Math.min(boundary, codeSpans.unclosedStart)
  if (firstUnclosedLt >= 0) boundary = Math.min(boundary, firstUnclosedLt)
  if (boundary < text.length) return boundary

  if (text.endsWith('\\')) return text.length - 1

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

  function drain() {
    const output = pending
    pending = ''
    heldSince = null
    return output
  }

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
          return [drain()]
        }
        return []
      }

      const output = pending.slice(0, boundary)
      pending = pending.slice(boundary)
      heldSince = pending.length > 0 ? (heldSince ?? now()) : null
      return output ? [output] : []
    },
    flush() {
      return drain()
    },
  }
}

exports.createMarkdownStreamBuffer = createMarkdownStreamBuffer
