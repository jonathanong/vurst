// Auto-generated platform dispatcher for @jongleberry/vurst.
//
// On first require, set ORT_DYLIB_PATH to the bundled onnxruntime shared
// library so `ort`'s load-dynamic feature can find it. Then load the platform
// .node binary.

const { existsSync } = require('node:fs')
const { join } = require('node:path')

if (!process.env.ORT_DYLIB_PATH) {
  const dylibByPlatform = {
    'darwin-arm64': 'libonnxruntime.dylib',
    'linux-x64': 'libonnxruntime.so',
    'linux-arm64': 'libonnxruntime.so',
  }
  const key = `${process.platform}-${process.arch}`
  const dylib = dylibByPlatform[key]
  if (dylib) {
    const candidate = join(__dirname, 'onnxruntime', key, dylib)
    if (existsSync(candidate)) {
      process.env.ORT_DYLIB_PATH = candidate
    }
  }
}

const { platform, arch } = process

function loadBinding() {
  switch (`${platform}-${arch}`) {
    case 'darwin-arm64':
      return require('./vurst.darwin-arm64.node')
    case 'linux-x64':
      return require('./vurst.linux-x64-gnu.node')
    case 'linux-arm64':
      return require('./vurst.linux-arm64-gnu.node')
    default:
      throw new Error(
        `Unsupported platform: ${platform}-${arch}. ` +
          `@jongleberry/vurst supports darwin-arm64, linux-x64 (glibc), and linux-arm64 (glibc).`,
      )
  }
}

const binding = loadBinding()

module.exports.chunk = binding.chunk
module.exports.sanitizeRssHtml = binding.sanitizeRssHtml
module.exports.sanitizeRssHtmlBatch = binding.sanitizeRssHtmlBatch
module.exports.detectAiGeneratedText = binding.detectAiGeneratedText
module.exports.extractDomRemovals = binding.extractDomRemovals
module.exports.applyDomRemovalsToHtml = binding.applyDomRemovalsToHtml
module.exports.extractMarkdownUrls = binding.extractMarkdownUrls
module.exports.htmlToEmbeddingText = binding.htmlToEmbeddingText
module.exports.renderMarkdownToHtml = binding.renderMarkdownToHtml
module.exports.renderMarkdownToHtmlBatch = binding.renderMarkdownToHtmlBatch
module.exports.sanitizePromptInjection = binding.sanitizePromptInjection
module.exports.getContentFromHtml = binding.getContentFromHtml
