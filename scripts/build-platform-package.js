#!/usr/bin/env node

"use strict";

const fs = require("node:fs");
const path = require("node:path");

const { PLATFORMS, VERSION } = require("./npm-platforms");

const [, , platformId, binarySource] = process.argv;

if (!platformId || !binarySource) {
  console.error("usage: node scripts/build-platform-package.js <platform-id> <binary-source>");
  process.exit(1);
}

const platform = PLATFORMS.find((entry) => entry.id === platformId);

if (!platform) {
  console.error(`unknown platform id: ${platformId}`);
  process.exit(1);
}

const packageRoot = path.resolve(__dirname, "..", "npm", platform.packageDir);
const binDir = path.join(packageRoot, "bin");
const binaryTarget = path.join(binDir, platform.binaryName);
const readmePath = path.join(packageRoot, "README.md");
const packageJsonPath = path.join(packageRoot, "package.json");

fs.mkdirSync(binDir, { recursive: true });
fs.copyFileSync(path.resolve(binarySource), binaryTarget);

if (platform.os !== "win32") {
  fs.chmodSync(binaryTarget, 0o755);
}

const packageJson = {
  name: platform.packageName,
  version: VERSION,
  description: `Prebuilt ${platform.os}/${platform.cpu} binary for minefit.`,
  license: "MIT",
  repository: {
    type: "git",
    url: "git+https://github.com/Barkeydog/minefit.git",
  },
  homepage: "https://github.com/Barkeydog/minefit#readme",
  bugs: {
    url: "https://github.com/Barkeydog/minefit/issues",
  },
  os: [platform.os],
  cpu: [platform.cpu],
  preferUnplugged: true,
  files: ["bin", "README.md", "LICENSE"],
  publishConfig: {
    access: "public",
  },
};

fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);

const readme = `# ${platform.packageName}

Prebuilt native binary for \`minefit\` on \`${platform.os}/${platform.cpu}\`.

This package is published as an internal install target for the top-level \`minefit\` npm package.
Most users should install \`minefit\`, not this package directly.
`;

fs.writeFileSync(readmePath, readme);

const licenseSource = path.resolve(__dirname, "..", "LICENSE");
const licenseTarget = path.join(packageRoot, "LICENSE");
fs.copyFileSync(licenseSource, licenseTarget);

console.log(`built ${platform.packageName} at ${packageRoot}`);
