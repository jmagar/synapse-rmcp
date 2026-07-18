"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const {
  binaryVersion,
  downloadUrl,
  releaseBaseUrl,
  releaseVersion,
  packageVersion,
  targetFor,
} = require("../lib/platform");

test("maps linux platform to release asset", () => {
  assert.deepEqual(targetFor("linux", "x64"), {
    asset: "synapse-x86_64.tar.gz",
    binary: "synapse",
  });
});

test("rejects unsupported platforms", () => {
  assert.throws(() => targetFor("darwin", "arm64"), /Unsupported platform/);
  assert.throws(() => targetFor("win32", "x64"), /Unsupported platform/);
});

test("falls back to the package version as the binary tag", () => {
  assert.equal(binaryVersion(), packageVersion());
  assert.equal(releaseVersion({}), `v${packageVersion()}`);
  assert.equal(Object.hasOwn(require("../package.json"), "binaryVersion"), false);
});

test("allows release tag and repo overrides", () => {
  const env = { SYNAPSE_RMCP_BINARY_VERSION: "v9.9.9", SYNAPSE_RMCP_REPO: "example/synapse-rmcp" };
  assert.equal(releaseBaseUrl(env), "https://github.com/example/synapse-rmcp/releases/download");
  assert.equal(downloadUrl(targetFor("linux", "x64"), env), "https://github.com/example/synapse-rmcp/releases/download/v9.9.9/synapse-x86_64.tar.gz");
});
