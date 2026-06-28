import assert from "node:assert/strict";
import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { test } from "node:test";

import { ShellUse, sessions } from "../dist/index.js";

const BIN = process.env.SHELL_USE_BIN;
const skip = !BIN;

test("echo roundtrip drives a real session", { skip }, async () => {
  const home = mkdtempSync(join(tmpdir(), "shell-use-js-"));
  const session = `nodetest-${process.pid}-a`;
  const su = new ShellUse(session, { home });
  try {
    await su.open();
    await su.submit("echo hello-sdk");
    await su.waitCommand();
    await su.expectText("hello-sdk", { strict: false });
    await su.expectExitCode(0);
    const state = await su.state();
    assert.ok(state.cols > 0);
  } finally {
    await su.close();
  }
});

test("sessions lists an open session", { skip }, async () => {
  const home = mkdtempSync(join(tmpdir(), "shell-use-js-"));
  const session = `nodetest-${process.pid}-b`;
  const su = new ShellUse(session, { home });
  await su.open();
  try {
    const names = await sessions({ home });
    assert.ok(names.includes(session));
  } finally {
    await su.close();
  }
});

test("snapshot lands in the client cwd", { skip }, async () => {
  const home = mkdtempSync(join(tmpdir(), "shell-use-js-"));
  const snapRoot = mkdtempSync(join(tmpdir(), "shell-use-snap-"));
  const name = `snap-${basename(snapRoot)}`;
  const original = process.cwd();
  const session = `nodetest-${process.pid}-c`;
  const su = new ShellUse(session, { home });
  try {
    await su.open();
    await su.submit("echo snapshot-marker");
    await su.waitCommand();
    process.chdir(snapRoot);
    const status = await su.expectSnapshot(name);
    assert.equal(status, "written");
    assert.ok(existsSync(join(snapRoot, "__snapshots__", `${name}.snap`)));
    assert.ok(!existsSync(join(original, "__snapshots__", `${name}.snap`)));
    assert.equal(await su.expectSnapshot(name), "passed");
  } finally {
    process.chdir(original);
    await su.close();
  }
});
