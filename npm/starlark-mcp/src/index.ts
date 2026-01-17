#!/usr/bin/env node

import { spawnSync } from "child_process";
import { existsSync } from "fs";
import { join } from "path";

/**
 * Returns the path to the starlark-mcp binary for the current platform.
 */
function getExePath(): string {
  const arch = process.arch;
  let os = process.platform as string;
  let extension = "";

  // Map Node.js platform names to our package naming convention
  if (["win32", "cygwin"].includes(process.platform)) {
    os = "windows";
    extension = ".exe";
  }

  // Map Node.js arch names to our package naming convention
  let archName = arch;
  if (arch === "x64") {
    archName = "x64";
  } else if (arch === "arm64") {
    archName = "arm64";
  }

  const packageName = `@anthropic-ai/starlark-mcp-${os}-${archName}`;
  const binaryName = `starlark-mcp${extension}`;

  // Try to find the binary in the platform-specific package
  try {
    const packagePath = require.resolve(`${packageName}/package.json`);
    const binaryPath = join(packagePath, "..", "bin", binaryName);

    if (existsSync(binaryPath)) {
      return binaryPath;
    }
  } catch (e) {
    // Package not found, will try fallback
  }

  // Fallback: try to find binary in common locations
  const fallbackPaths = [
    join(__dirname, "..", "bin", binaryName),
    join(__dirname, "..", "..", "bin", binaryName),
  ];

  for (const fallbackPath of fallbackPaths) {
    if (existsSync(fallbackPath)) {
      return fallbackPath;
    }
  }

  throw new Error(
    `Could not find the starlark-mcp binary for ${os}-${archName}.\n` +
    `Tried package: ${packageName}\n` +
    `Please ensure your platform is supported or install the binary manually.\n` +
    `Supported platforms: linux-x64, linux-arm64, darwin-x64, darwin-arm64, windows-x64`
  );
}

/**
 * Main entry point - runs the starlark-mcp binary with all CLI arguments.
 */
function run(): void {
  const args = process.argv.slice(2);
  const binaryPath = getExePath();

  const result = spawnSync(binaryPath, args, {
    stdio: "inherit",
    env: process.env,
  });

  // Handle spawn errors (e.g., ENOENT if binary is not executable)
  if (result.error) {
    console.error(`Failed to execute starlark-mcp: ${result.error.message}`);
    process.exit(1);
  }

  process.exit(result.status ?? 1);
}

run();
