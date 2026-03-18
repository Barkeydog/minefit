#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const {
  currentPlatformPackage,
  packageBinaryPath,
} = require("../scripts/npm-platforms");

const packageRoot = path.resolve(__dirname, "..");
const manifestPath = path.join(packageRoot, "Cargo.toml");

function runBinary(binaryPath) {
  const result = spawnSync(binaryPath, process.argv.slice(2), {
    cwd: packageRoot,
    stdio: "inherit",
  });

  if (result.error) {
    console.error(`minefit: unable to launch binary: ${result.error.message}`);
    process.exit(1);
  }

  process.exit(result.status === null ? 1 : result.status);
}

function resolveInstalledBinary() {
  const platform = currentPlatformPackage();
  if (!platform) {
    return null;
  }

  const packageJsonPath = `${platform.packageName}/package.json`;

  try {
    const resolved = require.resolve(packageJsonPath, { paths: [packageRoot] });
    const platformRoot = path.dirname(resolved);
    const binaryPath = packageBinaryPath(platformRoot, platform);
    if (fs.existsSync(binaryPath)) {
      return binaryPath;
    }
  } catch {}

  return null;
}

const installedBinary = resolveInstalledBinary();

if (installedBinary) {
  runBinary(installedBinary);
}

if (!fs.existsSync(manifestPath)) {
  console.error(
    "minefit: no prebuilt binary package was installed for this platform, and no Cargo manifest was found for a source fallback.",
  );
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
    console.error(
      "minefit: no native binary package was found for this platform, so source fallback requires Rust and Cargo from https://rustup.rs/.",
    );
  } else {
    console.error(`minefit: unable to launch cargo: ${result.error.message}`);
  }
  process.exit(1);
}

process.exit(result.status === null ? 1 : result.status);
