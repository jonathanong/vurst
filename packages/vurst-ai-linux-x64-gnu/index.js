const { existsSync } = require("node:fs");
const { join } = require("node:path");

if (!process.env.ORT_DYLIB_PATH) {
  const library = join(__dirname, "onnxruntime", "libonnxruntime.so");
  if (existsSync(library)) process.env.ORT_DYLIB_PATH = library;
}

module.exports = require("./vurst-ai.linux-x64-gnu.node");
