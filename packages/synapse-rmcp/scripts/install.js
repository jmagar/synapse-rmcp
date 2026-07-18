#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const https = require("node:https");
const crypto = require("node:crypto");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const {
  binaryPath,
  downloadUrl,
  installRoot,
  releaseVersion,
  targetFor,
} = require("../lib/platform");

function log(message) {
  process.stderr.write(`synapse-rmcp: ${message}\n`);
}

const MAX_DOWNLOAD_BYTES = 100 * 1024 * 1024;
const MAX_REDIRECTS = 5;

function requireHttps(url) {
  const parsed = new URL(url);
  if (parsed.protocol !== "https:") {
    throw new Error(`refusing non-HTTPS download URL: ${url}`);
  }
  return parsed;
}

function download(url, destination, redirects = 0) {
  return new Promise((resolve, reject) => {
    let parsed;
    try {
      parsed = requireHttps(url);
    } catch (error) {
      reject(error);
      return;
    }
    const request = https.get(parsed, (response) => {
      if ([301, 302, 303, 307, 308].includes(response.statusCode)) {
        if (redirects >= MAX_REDIRECTS || !response.headers.location) {
          response.resume();
          reject(new Error("download exceeded redirect limit or omitted Location"));
          return;
        }
        const next = new URL(response.headers.location, parsed).toString();
        response.resume();
        download(next, destination, redirects + 1).then(resolve, reject);
        return;
      }

      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`download failed (${response.statusCode}) from ${url}`));
        return;
      }

      const declared = Number(response.headers["content-length"] || 0);
      if (declared > MAX_DOWNLOAD_BYTES) {
        response.resume();
        reject(new Error(`download exceeds ${MAX_DOWNLOAD_BYTES} byte limit`));
        return;
      }
      let received = 0;
      const file = fs.createWriteStream(destination, { mode: 0o600, flags: "wx" });
      response.on("data", (chunk) => {
        received += chunk.length;
        if (received > MAX_DOWNLOAD_BYTES) request.destroy(new Error("download exceeded byte limit"));
      });
      response.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", reject);
    });

    request.setTimeout(30_000, () => request.destroy(new Error("download timed out")));
    request.on("error", (error) => {
      fs.rmSync(destination, { force: true });
      reject(error);
    });
  });
}

function sha256(file) {
  return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

function verifyChecksum(archive, checksumFile) {
  const line = fs.readFileSync(checksumFile, "utf8").trim();
  const expected = line.match(/^([a-fA-F0-9]{64})(?:\s+\*?.+)?$/)?.[1]?.toLowerCase();
  if (!expected || sha256(archive) !== expected) throw new Error("release checksum verification failed");
}

function validateArchive(archive) {
  const result = spawnSync("tar", ["-tzvf", archive], { encoding: "utf8" });
  if (result.status !== 0) throw new Error("unable to inspect release archive");
  const entries = result.stdout.trim().split(/\r?\n/).filter(Boolean);
  if (entries.length !== 1 || !entries[0].startsWith("-") || !/\s(?:\.\/)?synapse$/.test(entries[0])) {
    throw new Error("release archive must contain exactly one regular file named synapse");
  }
}

function extract(archive, destination) {
  fs.mkdirSync(destination, { recursive: true });

  const result = spawnSync("tar", ["-xzf", archive, "-C", destination], {
    encoding: "utf8",
  });

  if (result.status !== 0) {
    throw new Error((result.stderr || result.stdout || "tar extraction failed").trim());
  }
}

async function main() {
  if (process.env.SYNAPSE_RMCP_SKIP_DOWNLOAD === "1") {
    log("skipping binary download because SYNAPSE_RMCP_SKIP_DOWNLOAD=1");
    return;
  }

  const target = targetFor();
  const destination = binaryPath();

  if (fs.existsSync(destination)) {
    log(`${path.basename(destination)} already installed for ${releaseVersion()}`);
    return;
  }

  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "synapse-rmcp-install-"));
  const archive = path.join(tempDir, target.asset);
  const checksum = `${archive}.sha256`;
  const staging = path.join(tempDir, "staging");

  try {
    const url = downloadUrl(target);
    log(`downloading ${url}`);
    await download(url, archive);
    await download(`${url}.sha256`, checksum);
    verifyChecksum(archive, checksum);
    validateArchive(archive);
    extract(archive, staging);
    fs.mkdirSync(installRoot(), { recursive: true });
    fs.chmodSync(path.join(staging, target.binary), 0o755);
    fs.renameSync(path.join(staging, target.binary), destination);
    log(`installed ${destination}`);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

if (require.main === module) {
  main().catch((error) => {
    log(error.message);
    process.exitCode = 1;
  });
}

module.exports = { download, requireHttps, sha256, validateArchive, verifyChecksum };
