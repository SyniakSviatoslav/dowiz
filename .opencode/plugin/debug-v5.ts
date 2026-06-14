import { writeFileSync, mkdirSync, rmSync, readFileSync, existsSync } from "node:fs"
import { join } from "node:path"
import { createHash } from "node:crypto"
import { EntropySensor } from "./entropy-sensor.ts"

const TMP = join(process.cwd(), ".opencode", "__test_workspace__")
rmSync(TMP, { recursive: true, force: true })
mkdirSync(join(TMP, ".opencode"), { recursive: true })
mkdirSync(join(TMP, "osc-test"), { recursive: true })

const f = join(TMP, "osc-test", "file.ts")

// Check: does readFileSync work with mixed separators?
const p = join(TMP, "osc-test/file.ts")
writeFileSync(f, "version-a\n")
console.log("file exists:", existsSync(p))
console.log("path =", p)
try {
  const content = readFileSync(p, "utf8")
  console.log("read OK:", content.trim())
} catch (e: any) {
  console.log("read FAILED:", e.message)
}

// Now test the plugin
const plugin = await EntropySensor({ directory: TMP })
const after = plugin["tool.execute.after"]

const sessionID = "trace-" + Date.now()

// Step 1
try {
  await after({ tool: "edit", sessionID }, { tool: "edit", args: { filePath: "osc-test/file.ts" } })
  console.log("after step1: OK")
} catch (e: any) {
  console.log("after step1: threw:", e.message)
}

// Step 2
writeFileSync(f, "version-b\n")
try {
  await after({ tool: "edit", sessionID }, { tool: "edit", args: { filePath: "osc-test/file.ts" } })
  console.log("after step2: OK")
} catch (e: any) {
  console.log("after step2: threw:", e.message)
}

// Step 3 - revert
writeFileSync(f, "version-a\n")
try {
  await after({ tool: "edit", sessionID }, { tool: "edit", args: { filePath: "osc-test/file.ts" } })
  console.log("after step3: OK")
} catch (e: any) {
  console.log("after step3: threw:", e.message)
}
