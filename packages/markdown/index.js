'use strict'

// Platform dispatcher for @jongleberry/vurst-markdown.

const { platform, arch } = process

function loadBinding() {
  switch (`${platform}-${arch}`) {
    case 'darwin-arm64':
      return require('./vurst-markdown.darwin-arm64.node')
    case 'linux-x64':
      return require('./vurst-markdown.linux-x64-gnu.node')
    case 'linux-arm64':
      return require('./vurst-markdown.linux-arm64-gnu.node')
    default:
      throw new Error(
        `Unsupported platform: ${platform}-${arch}. ` +
          `@jongleberry/vurst-markdown supports darwin-arm64, linux-x64 (glibc), and linux-arm64 (glibc).`,
      )
  }
}

const binding = loadBinding()

module.exports.chunk = binding.chunk
module.exports.renderMarkdownToHtml = binding.renderMarkdownToHtml
module.exports.renderMarkdownToHtmlBatch = binding.renderMarkdownToHtmlBatch
module.exports.extractMarkdownUrls = binding.extractMarkdownUrls
