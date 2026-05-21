'use strict'

// Platform dispatcher for @jongleberry/vurst-ai.
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
      return require('./vurst-ai.darwin-arm64.node')
    case 'linux-x64':
      return require('./vurst-ai.linux-x64-gnu.node')
    case 'linux-arm64':
      return require('./vurst-ai.linux-arm64-gnu.node')
    default:
      throw new Error(
        `Unsupported platform: ${platform}-${arch}. ` +
          `@jongleberry/vurst-ai supports darwin-arm64, linux-x64 (glibc), and linux-arm64 (glibc).`,
      )
  }
}

const binding = loadBinding()

module.exports.detectAiGeneratedText = binding.detectAiGeneratedText
