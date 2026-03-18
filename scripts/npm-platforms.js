"use strict";

const path = require("node:path");

const VERSION = "0.7.7";

const PLATFORMS = [
  {
    id: "win32-x64-msvc",
    packageName: "@barkey/minefit-win32-x64-msvc",
    packageDir: "barkey-minefit-win32-x64-msvc",
    os: "win32",
    cpu: "x64",
    target: "x86_64-pc-windows-msvc",
    binaryName: "minefit.exe",
    archiveName: `minefit-v${VERSION}-x86_64-pc-windows-msvc.zip`,
  },
  {
    id: "darwin-arm64",
    packageName: "@barkey/minefit-darwin-arm64",
    packageDir: "barkey-minefit-darwin-arm64",
    os: "darwin",
    cpu: "arm64",
    target: "aarch64-apple-darwin",
    binaryName: "minefit",
    archiveName: `minefit-v${VERSION}-aarch64-apple-darwin.tar.gz`,
  },
  {
    id: "darwin-x64",
    packageName: "@barkey/minefit-darwin-x64",
    packageDir: "barkey-minefit-darwin-x64",
    os: "darwin",
    cpu: "x64",
    target: "x86_64-apple-darwin",
    binaryName: "minefit",
    archiveName: `minefit-v${VERSION}-x86_64-apple-darwin.tar.gz`,
  },
  {
    id: "linux-x64-gnu",
    packageName: "@barkey/minefit-linux-x64-gnu",
    packageDir: "barkey-minefit-linux-x64-gnu",
    os: "linux",
    cpu: "x64",
    target: "x86_64-unknown-linux-gnu",
    binaryName: "minefit",
    archiveName: `minefit-v${VERSION}-x86_64-unknown-linux-gnu.tar.gz`,
  },
];

function currentPlatformPackage() {
  return PLATFORMS.find(
    (platform) => platform.os === process.platform && platform.cpu === process.arch,
  );
}

function packageBinaryPath(root, platform) {
  return path.join(root, "bin", platform.binaryName);
}

module.exports = {
  VERSION,
  PLATFORMS,
  currentPlatformPackage,
  packageBinaryPath,
};
