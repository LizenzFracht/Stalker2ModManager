import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { test } from "node:test";
import { findBundle } from "./release-files.mjs";

test("release packaging selects the requested version rather than a stale bundle", () => {
  const dir = mkdtempSync(join(tmpdir(), "s2mm-release-test-"));
  try {
    for (const name of ["S2MM_0.1.0_x64-setup.exe", "S2MM_0.1.1_x64-setup.exe", "S2MM_0.1.10_x64-setup.exe"]) writeFileSync(join(dir, name), "test");
    assert.equal(basename(findBundle(dir, "0.1.1", "-setup.exe")), "S2MM_0.1.1_x64-setup.exe");
    assert.throws(() => findBundle(dir, "0.2.0", "-setup.exe"), /0\.2\.0/);
  } finally {
    assert.equal(dirname(dir), tmpdir());
    assert.ok(basename(dir).startsWith("s2mm-release-test-"));
    rmSync(dir, { recursive: true });
  }
});
