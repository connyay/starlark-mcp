#!/usr/bin/env node

import { spawnSync } from "child_process";
import { join } from "path";

const isWindows = process.platform === "win32";
const os = isWindows ? "windows" : process.platform;
const ext = isWindows ? ".exe" : "";
const pkg = `@connyay/starlark-mcp-${os}-${process.arch}`;

let binaryPath: string;
try {
  const pkgPath = require.resolve(`${pkg}/package.json`);
  binaryPath = join(pkgPath, "..", "bin", `starlark-mcp${ext}`);
} catch {
  console.error(
    `Unsupported platform: ${os}-${process.arch}\n` +
    `Supported: linux-x64, linux-arm64, darwin-x64, darwin-arm64, windows-x64`
  );
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`Failed to execute starlark-mcp: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
