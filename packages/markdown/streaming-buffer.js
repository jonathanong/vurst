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

  let index = start
  while (index < text.length) {
    const codeSpan = codeSpans[codeSpanIndex]
    if (codeSpan !== undefined && index === codeSpan.start) {
      index = codeSpan.end
      codeSpanIndex += 1
      continue
    }
    if (text[index] === '\\') {
      index += 2
      continue
    }
    if (text[index] === open) {
      depth += 1
    } else if (text[index] === close) {
      depth -= 1
      if (depth === 0) return index
    }
    index += 1
  }
  return -1
}

function findLinkDestinationEnd(text, start) {
  let depth = 0
  let quote = null
  let titleMayStart = false
  let index = start
  while (index < text.length) {
    const character = text[index]
    if (character === '\\') {
      index += 2
      continue
    }
    if (quote !== null) {
      if (character === quote) quote = null
      index += 1
      continue
    }
    if (character === ' ' || character === '\t' || character === '\n') {
      titleMayStart = true
      index += 1
      continue
    }
    if (titleMayStart && (character === '"' || character === "'")) {
      quote = character
      titleMayStart = false
    } else if (character === '(') {
      titleMayStart = false
      depth += 1
    } else if (character === ')') {
      depth -= 1
      if (depth === 0) return index
    } else {
      titleMayStart = false
    }
    index += 1
  }
  return -1
}

function advanceCodeSpanIndex(codeSpans, codeSpanIndex, index) {
  while (codeSpanIndex < codeSpans.length && codeSpans[codeSpanIndex].end <= index) {
    codeSpanIndex += 1
  }
  return codeSpanIndex
}

function inspectLink(text, index, codeSpans) {
  const labelEnd = findBalancedEnd(text, index, '[', ']', codeSpans)
  const opener =
    index > 0 && text[index - 1] === '!' && !isEscaped(text, index - 1) ? index - 1 : index
  if (labelEnd === -1 || labelEnd === text.length - 1) {
    return { opener, end: -1, nextSearchStart: index + 1 }
  }
  if (text[labelEnd + 1] !== '(') {
    return { opener: null, end: -1, nextSearchStart: labelEnd + 1 }
  }

  const destinationEnd = findLinkDestinationEnd(text, labelEnd + 1)
  return {
    opener,
    end: destinationEnd === -1 ? -1 : destinationEnd + 1,
    nextSearchStart: destinationEnd === -1 ? labelEnd + 1 : destinationEnd + 1,
  }
}

function findLinkRanges(text, codeSpans) {
  const complete = []
  const unsafe = []
  let searchStart = 0
  let codeSpanIndex = 0
  while (searchStart < text.length) {
    const index = text.indexOf('[', searchStart)
    if (index === -1) break

    codeSpanIndex = advanceCodeSpanIndex(codeSpans, codeSpanIndex, index)
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

    const link = inspectLink(text, index, codeSpans)
    if (link.opener !== null && link.end === -1) unsafe.push(link.opener)
    if (link.end >= 0) complete.push({ start: link.opener, end: link.end })
    searchStart = link.nextSearchStart
  }
  return { complete, unsafe }
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

function findHtmlTagEnd(text, opener) {
  let quote = null
  let index = opener + 1
  while (index < text.length) {
    const character = text[index]
    if (quote !== null) {
      if (character === quote) quote = null
      index += 1
      continue
    }
    if (character === '"' || character === "'") {
      quote = character
    } else if (character === '>') {
      return index
    } else if (character === '<') {
      return -1
    }
    index += 1
  }
  return null
}

function findHtmlTagRanges(text, codeSpans) {
  const complete = []
  const unsafe = []
  let codeSpanIndex = 0
  let searchStart = 0
  while (searchStart < text.length) {
    const index = text.indexOf('<', searchStart)
    if (index === -1) break

    codeSpanIndex = advanceCodeSpanIndex(codeSpans, codeSpanIndex, index)
    const codeSpan = codeSpans[codeSpanIndex]
    if (codeSpan !== undefined && codeSpan.start < index) {
      searchStart = codeSpan.end
      codeSpanIndex += 1
      continue
    }

    const tagEnd = findHtmlTagEnd(text, index)
    if (tagEnd === null) {
      unsafe.push(index)
      break
    }
    if (tagEnd === -1) {
      searchStart = index + 1
      continue
    }
    complete.push({ start: index, end: tagEnd + 1 })
    searchStart = tagEnd + 1
  }
  return { complete, unsafe }
}

function isInsideCompleteRange(index, ranges) {
  return ranges.some((range) => range.start <= index && index < range.end)
}

function findFirstTopLevelUnsafe(candidates, enclosingRanges) {
  let rangeIndex = 0
  for (const candidate of candidates) {
    while (
      rangeIndex < enclosingRanges.length &&
      enclosingRanges[rangeIndex].end <= candidate
    ) {
      rangeIndex += 1
    }
    const range = enclosingRanges[rangeIndex]
    if (range === undefined || candidate < range.start) return candidate
  }
  return -1
}

function findSafeBoundary(text) {
  const codeSpans = findCodeSpans(text)
  const opaqueCodeSpans = [...codeSpans.complete]
  if (codeSpans.unclosedStart >= 0) {
    opaqueCodeSpans.push({ start: codeSpans.unclosedStart, end: text.length })
  }

  const links = findLinkRanges(text, opaqueCodeSpans)
  const htmlTags = findHtmlTagRanges(text, opaqueCodeSpans)
  const completeOuterRanges = [...links.complete, ...htmlTags.complete]
  const unclosedCodeIsNested =
    codeSpans.unclosedStart >= 0 &&
    isInsideCompleteRange(codeSpans.unclosedStart, completeOuterRanges)
  const firstUnsafeLink = findFirstTopLevelUnsafe(links.unsafe, htmlTags.complete)
  const firstUnsafeHtmlTag = findFirstTopLevelUnsafe(htmlTags.unsafe, links.complete)

  let boundary = text.length
  if (firstUnsafeLink >= 0) boundary = Math.min(boundary, firstUnsafeLink)
  if (codeSpans.unclosedStart >= 0 && !unclosedCodeIsNested) {
    boundary = Math.min(boundary, codeSpans.unclosedStart)
  }
  if (firstUnsafeHtmlTag >= 0) boundary = Math.min(boundary, firstUnsafeHtmlTag)
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

  function emitSafePrefix(output) {
    if (pending.length === 0) {
      heldSince = null
      return [output]
    }
    if (heldSince === null) heldSince = now()
    if (now() - heldSince >= maxHoldMs) return [output, drain()]
    return [output]
  }

  return {
    push(text) {
      if (typeof text !== 'string') {
        throw new TypeError('text must be a string')
      }

      pending += text
      if (pending.length === 0) return []
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
      return emitSafePrefix(output)
    },
    flush() {
      return drain()
    },
  }
}

exports.createMarkdownStreamBuffer = createMarkdownStreamBuffer
