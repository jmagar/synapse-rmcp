import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const tokenSheet = readFileSync(join(root, "components/aurora.css"), "utf8");

function sourceFiles(path: string): string[] {
  return readdirSync(path).flatMap((name) => {
    const child = join(path, name);
    if (statSync(child).isDirectory()) return sourceFiles(child);
    return /\.(css|tsx?)$/.test(name) ? [child] : [];
  });
}

describe("Aurora token contract", () => {
  it("loads the canonical token sheet", () => {
    const globals = readFileSync(join(root, "app/globals.css"), "utf8");
    expect(globals).toContain('@import "../components/aurora.css"');
  });

  it("defines every Aurora variable referenced by application sources", () => {
    const definitions = new Set(
      [...tokenSheet.matchAll(/(--aurora-[\w-]+)\s*:/g)].map((match) => match[1]),
    );
    const references = new Set<string>();
    const applicationFiles = ["app", "components", "lib"].flatMap((directory) =>
      sourceFiles(join(root, directory)),
    );
    for (const file of applicationFiles) {
      if (file.endsWith("components/aurora.css")) continue;
      const source = readFileSync(file, "utf8");
      for (const match of source.matchAll(/var\((--aurora-[\w-]+)/g)) references.add(match[1]);
    }
    expect([...references].filter((token) => !definitions.has(token))).toEqual([]);
  });
});
