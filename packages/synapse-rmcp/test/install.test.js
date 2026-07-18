"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const test = require("node:test");

const { requireHttps, sha256, validateArchive, verifyChecksum } = require("../scripts/install");

test("installer refuses plaintext and downgrade URLs", () => {
  assert.throws(() => requireHttps("http://example.test/synapse.tar.gz"), /non-HTTPS/);
  assert.equal(requireHttps("https://example.test/synapse.tar.gz").protocol, "https:");
});

test("checksum verification fails closed", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "synapse-install-test-"));
  try {
    const archive = path.join(dir, "asset");
    const checksum = `${archive}.sha256`;
    fs.writeFileSync(archive, "trusted bytes");
    fs.writeFileSync(checksum, `${sha256(archive)}  asset\n`);
    assert.doesNotThrow(() => verifyChecksum(archive, checksum));
    fs.writeFileSync(archive, "tampered bytes");
    assert.throws(() => verifyChecksum(archive, checksum), /checksum/);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("archive validation rejects extra and traversal entries", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "synapse-archive-test-"));
  try {
    fs.writeFileSync(path.join(dir, "synapse"), "binary");
    fs.writeFileSync(path.join(dir, "extra"), "unexpected");
    const good = path.join(dir, "good.tar.gz");
    const bad = path.join(dir, "bad.tar.gz");
    execFileSync("tar", ["-czf", good, "synapse"], { cwd: dir });
    execFileSync("tar", ["-czf", bad, "synapse", "extra"], { cwd: dir });
    assert.doesNotThrow(() => validateArchive(good));
    assert.throws(() => validateArchive(bad), /exactly one regular file/);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
