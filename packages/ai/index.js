// Hand-maintained wrapper around the NAPI-RS generated binding (./binding.js).
//
// `napi build --js binding.js --dts binding.d.ts` regenerates binding.js on
// every build, so it must never contain hand-written logic -- anything added
// there is silently lost on the next build. The ONNX Runtime dylib
// resolution below must run *before* the native binding is required, so it
// lives here instead, where a rebuild can't clobber it.

const { existsSync } = require('node:fs')
const { join, dirname } = require('node:path')

// process.platform + process.arch -> ONNX Runtime shared library file name.
const dylibByPlatform = {
  'darwin-arm64': 'libonnxruntime.dylib',
  'linux-x64': 'libonnxruntime.so',
  'linux-arm64': 'libonnxruntime.so',
}

// process.platform + process.arch -> published per-platform package's napi platformArchABI.
const platformArchABIByKey = {
  'darwin-arm64': 'darwin-arm64',
  'linux-x64': 'linux-x64-gnu',
  'linux-arm64': 'linux-arm64-gnu',
}

function resolveOrtDylibPath(key, dylib) {
  // Local/dev build: onnxruntime staged next to this file (see ci.yml, release.yml).
  const devCandidate = join(__dirname, 'onnxruntime', key, dylib)
  if (existsSync(devCandidate)) {
    return devCandidate
  }

  // Published package: the ONNX Runtime dylib is bundled inside the
  // per-platform @jongleberry/vurst-ai-<platform> optionalDependency, not
  // this meta package.
  const platformArchABI = platformArchABIByKey[key]
  if (!platformArchABI) {
    return null
  }
  try {
    const platformPackageJsonPath = require.resolve(
      `@jongleberry/vurst-ai-${platformArchABI}/package.json`,
    )
    const publishedCandidate = join(dirname(platformPackageJsonPath), 'onnxruntime', key, dylib)
    if (existsSync(publishedCandidate)) {
      return publishedCandidate
    }
  } catch {
    // Per-platform package isn't installed (unsupported platform, or the
    // optionalDependency failed to install). Fall through and let the
    // native binding surface its own "ONNX Runtime dylib" error.
  }

  return null
}

if (!process.env.ORT_DYLIB_PATH) {
  const key = `${process.platform}-${process.arch}`
  const dylib = dylibByPlatform[key]
  if (dylib) {
    const dylibPath = resolveOrtDylibPath(key, dylib)
    if (dylibPath) {
      process.env.ORT_DYLIB_PATH = dylibPath
    }
  }
}

const binding = require('./binding.js')

module.exports = binding
module.exports.detectAiGeneratedText = binding.detectAiGeneratedText
