#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const packageRoot = path.resolve(__dirname, "..");
const manifestPath = path.join(packageRoot, "Cargo.toml");

if (!fs.existsSync(manifestPath)) {
  console.error(`minefit: missing Cargo manifest at ${manifestPath}`);
  process.exit(1);
}

const args = [
  "run",
  "--release",
  "-p",
  "minefit",
  "--manifest-path",
  manifestPath,
  "--",
  ...process.argv.slice(2),
];

const result = spawnSync("cargo", args, {
  cwd: packageRoot,
  stdio: "inherit",
});

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error("minefit: Rust and Cargo are required. Install them from https://rustup.rs/.");
  } else {
    console.error(`minefit: unable to launch cargo: ${result.error.message}`);
  }
  process.exit(1);
}

process.exit(result.status === null ? 1 : result.status);
