// DeliveryOS harness: entropy sensor (A4/A5) + Karpathy guards (P1/P3).
// Tests use node:test + tsx. No opencode runtime required — we call the hook
// objects directly, simulating the Plugin API handshake.

import { test, describe, before, after } from "node:test"
import assert from "node:assert"
import { existsSync, mkdirSync, writeFileSync, rmSync, readFileSync } from "node:fs"
import { join } from "node:path"
import { EntropySensor } from "./entropy-sensor.ts"
import { KarpathyGuards } from "./karpathy-guards.ts"
import { createHash } from "node:crypto"

const TMP = join(process.cwd(), ".opencode", "__test_workspace__")
const sha = (s: string) => createHash("sha1").update(s).digest("hex").slice(0, 12)

function resetDir() {
  rmSync(TMP, { recursive: true, force: true })
  mkdirSync(TMP, { recursive: true })
  mkdirSync(join(TMP, ".opencode"), { recursive: true })
}

describe("EntropySensor — A4 permission enforcement", () => {
  before(() => resetDir())
  after(() => rmSync(TMP, { recursive: true, force: true }))

  test("V1: blocks edit to read-only glob path", async () => {
    // Write a fake read-only file
    const ro = join(TMP, "migrations", "001-test.ts")
    mkdirSync(join(TMP, "migrations"), { recursive: true })
    writeFileSync(ro, "// read-only")
    writeFileSync(join(TMP, ".opencode", "entropy.json"), JSON.stringify({
      readOnlyGlobs: ["migrations/"],
      fileMutationTools: ["edit", "write", "patch"],
      noProgressHalt: 99,
      noProgressWarn: 99,
      sameFileWarn: 99,
      oscillationWindow: 6,
      hardToolBudget: 999,
      progressCmds: ["\\btsc\\b"],
      episodeDir: ".opencode/episodes",
    }))

    const plugin = await EntropySensor({ directory: TMP })
    const before = plugin["tool.execute.before"]!
    let err: Error | null = null
    try {
      await before({ tool: "edit", sessionID: "v1-test" }, { args: { filePath: "migrations/001-test.ts" } })
    } catch (e: any) {
      err = e
    }
    assert.ok(err, "A4 should throw for read-only path")
    assert.match(err!.message, /read-only/)
  })

  test("V2: allows edit to non-read-only path", async () => {
    const plugin = await EntropySensor({ directory: TMP })
    const before = plugin["tool.execute.before"]!
    let err: Error | null = null
    try {
      await before({ tool: "edit", sessionID: "v2-test" }, { args: { filePath: "apps/web/src/Component.tsx" } })
    } catch (e: any) {
      err = e
    }
    assert.strictEqual(err, null, "should not block non-read-only paths")
  })
})

describe("EntropySensor — A5 entropy sensor", () => {
  before(() => resetDir())
  after(() => rmSync(TMP, { recursive: true, force: true }))

  test("V3: warns at noProgressWarn threshold, halts at noProgressHalt", async () => {
    const plugin = await EntropySensor({ directory: TMP })
    const before = plugin["tool.execute.before"]!
    const after = plugin["tool.execute.after"]!

    // Need a distinct session so it starts fresh
    const sessionID = "v3-loop-" + Date.now()

    // Fire tools up to warn threshold
    let warns = 0
    for (let i = 0; i < 14; i++) {
      try {
        await before({ tool: "bash", sessionID, name: "bash" }, { args: { command: "echo hi" } })
        await after({ tool: "bash", sessionID },
          { tool: "bash", args: { command: "echo hi" }, result: { exitCode: 0 } })
      } catch (e: any) {
        // should not warn at 14
      }
    }

    // 15th — should warn
    try {
      await before({ tool: "bash", sessionID }, { args: { command: "echo hi" } })
    } catch (e: any) {
      if (e.message?.includes("WARN")) warns++
    }

    // Fire more up to halt threshold (25)
    for (let i = 0; i < 10; i++) {
      try {
        await before({ tool: "bash", sessionID }, { args: { command: "echo hi" } })
      } catch (e: any) {
        if (e.message?.includes("WARN")) warns++
      }
    }

    // 25th — should halt
    let halted = false
    try {
      await before({ tool: "bash", sessionID }, { args: { command: "echo hi" } })
    } catch (e: any) {
      if (e.message?.includes("HALT")) halted = true
    }

    assert.ok(warns >= 1, "should have warned at least once")
    assert.ok(halted, "should halt at noProgressHalt threshold")
  })

  test("V4: progress heartbeat resets counters", async () => {
    const plugin = await EntropySensor({ directory: TMP })
    const before = plugin["tool.execute.before"]!
    const after = plugin["tool.execute.after"]!
    const sessionID = "v4-progress-" + Date.now()

    // Run 10 non-progress calls
    for (let i = 0; i < 10; i++) {
      await before({ tool: "bash", sessionID }, { args: { command: "echo hi" } })
      await after({ tool: "bash", sessionID }, { args: { command: "echo hi" }, result: { exitCode: 0 } })
    }

    // Progress signal: tsc passes
    try {
      await before({ tool: "bash", sessionID, args: { command: "npx tsc --noEmit" } },
        { args: { command: "npx tsc --noEmit" } })
    } catch { /* may or may not */ }
    await after({ tool: "bash", sessionID, args: { command: "npx tsc --noEmit" } },
      { title: "", output: "", metadata: { exit: 0 } })

    // After reset, 14 more should not warn (since progress reset counter to 0)
    let hit = false
    for (let i = 0; i < 14; i++) {
      try {
        await before({ tool: "bash", sessionID }, { args: { command: "echo hi" } })
      } catch (e: any) {
        if (e.message?.includes("WARN")) hit = true
      }
    }
    assert.ok(!hit, "should NOT warn after progress reset within window")
  })

  test("V5: oscillation detection — edit->revert->edit content hash", async () => {
    const plugin = await EntropySensor({ directory: TMP })
    const before = plugin["tool.execute.before"]!
    const after = plugin["tool.execute.after"]!

    mkdirSync(join(TMP, "osc-test"), { recursive: true })
    const f = join(TMP, "osc-test", "file.ts")
    writeFileSync(f, "version-a\n")

    const sessionID = "v5-osc-" + Date.now()

    // Write version A
    writeFileSync(f, "version-a\n")
    await after({ tool: "edit", sessionID, args: { filePath: "osc-test/file.ts" } },
      { title: "edit done", output: "", metadata: {} })

    // Write version B (different)
    writeFileSync(f, "version-b\n")
    await after({ tool: "edit", sessionID, args: { filePath: "osc-test/file.ts" } },
      { title: "edit done", output: "", metadata: {} })

    // Revert to A (oscillation!)
    writeFileSync(f, "version-a\n")
    let oscillated = false
    try {
      await after({ tool: "edit", sessionID, args: { filePath: "osc-test/file.ts" } },
        { title: "edit done", output: "", metadata: {} })
    } catch (e: any) {
      if (e.message?.includes("oscillat")) oscillated = true
    }

    assert.ok(oscillated, "should detect content oscillation (A->B->A)")
  })
})

describe("KarpathyGuards — P1 plan-gate", () => {
  before(() => resetDir())
  after(() => rmSync(TMP, { recursive: true, force: true }))

  test("V1: blocks edit when no plan file exists", async () => {
    writeFileSync(join(TMP, ".opencode", "karpathy.json"), JSON.stringify({
      planRequired: true,
      fileMutationTools: ["edit", "write", "patch"],
      skipScopeWhenNoScopeFile: true,
    }))

    const plugin = await KarpathyGuards({ directory: TMP })
    const before = plugin["tool.execute.before"]!
    let err: Error | null = null
    try {
      await before({ tool: "edit", sessionID: "kp-v1" })
    } catch (e: any) {
      err = e
    }
    assert.ok(err, "P1 should throw when no plan file")
    assert.match(err!.message, /plan/i)
  })

  test("V2: allows edit when plan file exists", async () => {
    writeFileSync(join(TMP, ".opencode", "plan.jsonc"), JSON.stringify({
      approach: "test",
      files: ["test.ts"],
    }))
    const plugin = await KarpathyGuards({ directory: TMP })
    const before = plugin["tool.execute.before"]!
    let err: Error | null = null
    try {
      await before({ tool: "edit", sessionID: "kp-v2", args: { filePath: "test.ts" } })
    } catch (e: any) {
      err = e
    }
    assert.strictEqual(err, null, "P1 should not block when plan exists")
  })
})

describe("KarpathyGuards — P3 scope-block", () => {
  before(() => resetDir())
  after(() => rmSync(TMP, { recursive: true, force: true }))

  test("V3: allows edit when no scope file exists", async () => {
    writeFileSync(join(TMP, ".opencode", "plan.jsonc"), "{}")
    writeFileSync(join(TMP, ".opencode", "karpathy.json"), JSON.stringify({
      planRequired: false,
      skipScopeWhenNoScopeFile: true,
    }))

    const plugin = await KarpathyGuards({ directory: TMP })
    const before = plugin["tool.execute.before"]!
    let err: Error | null = null
    try {
      await before({ tool: "edit", sessionID: "kp-v3", args: { filePath: "apps/web/src/any.ts" } })
    } catch (e: any) {
      err = e
    }
    assert.strictEqual(err, null, "should pass when no scope file")
  })

  test("V4: blocks edit outside scope", async () => {
    writeFileSync(join(TMP, ".opencode", "scope.jsonc"), JSON.stringify({
      scopes: ["apps/api/"],
    }))

    const plugin = await KarpathyGuards({ directory: TMP })
    const before = plugin["tool.execute.before"]!
    let err: Error | null = null
    try {
      await before({ tool: "edit", sessionID: "kp-v4", args: { filePath: "apps/web/src/outside.ts" } })
    } catch (e: any) {
      err = e
    }
    assert.ok(err, "P3 should block outside-scope edits")
  })

  test("V5: allows edit inside scope", async () => {
    const plugin = await KarpathyGuards({ directory: TMP })
    const before = plugin["tool.execute.before"]!
    let err: Error | null = null
    try {
      await before({ tool: "edit", sessionID: "kp-v5", args: { filePath: "apps/api/src/routes/ok.ts" } })
    } catch (e: any) {
      err = e
    }
    assert.strictEqual(err, null, "should pass when inside scope")
  })
})
