'use strict'

// Platform dispatcher for @jongleberry/vurst-html.

const { platform, arch } = process

function loadBinding() {
  switch (`${platform}-${arch}`) {
    case 'darwin-arm64':
      return require('./vurst-html.darwin-arm64.node')
    case 'linux-x64':
      return require('./vurst-html.linux-x64-gnu.node')
    case 'linux-arm64':
      return require('./vurst-html.linux-arm64-gnu.node')
    default:
      throw new Error(
        `Unsupported platform: ${platform}-${arch}. ` +
          `@jongleberry/vurst-html supports darwin-arm64, linux-x64 (glibc), and linux-arm64 (glibc).`,
      )
  }
}

const binding = loadBinding()

module.exports.sanitizeRssHtml = binding.sanitizeRssHtml
module.exports.sanitizeRssHtmlBatch = binding.sanitizeRssHtmlBatch
module.exports.htmlToEmbeddingText = binding.htmlToEmbeddingText
module.exports.extractDomRemovals = binding.extractDomRemovals
module.exports.applyDomRemovalsToHtml = binding.applyDomRemovalsToHtml
module.exports.getContentFromHtml = binding.getContentFromHtml
module.exports.sanitizePromptInjection = binding.sanitizePromptInjection
