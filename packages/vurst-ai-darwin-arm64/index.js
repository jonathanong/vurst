const { existsSync } = require("node:fs");
const { join } = require("node:path");

if (!process.env.ORT_DYLIB_PATH) {
  const library = join(__dirname, "onnxruntime", "libonnxruntime.dylib");
  if (existsSync(library)) process.env.ORT_DYLIB_PATH = library;
}

module.exports = require("./vurst-ai.darwin-arm64.node");
