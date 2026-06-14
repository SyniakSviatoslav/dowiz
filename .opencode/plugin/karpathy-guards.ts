// Karpathy Working Rules — model-agnostic guard hooks.
// Principle 1 (Think Before): plan-gate — plan.jsonc must exist before tool.execute
// Principle 2 (Simplicity): deferred to eslint rules + churn-report script
// Principle 3 (Surgical): scope-block — skip edits to files outside scope.jsonc
// Principle 4 (Goal-Driven): deferred — already enforced by existing harness gates
// Field shapes verified against opencode 1.16.2 + @opencode-ai/plugin 1.15.11.

import type { Plugin } from "@opencode-ai/plugin"
import { existsSync, readFileSync } from "node:fs"
import { join } from "node:path"

interface KarpathyConfig {
  fileMutationTools: string[]
  planRequired: boolean
  planName: string
  scopeName: string
  skipScopeWhenNoScopeFile: boolean
}

const DEFAULT: KarpathyConfig = {
  fileMutationTools: ["edit", "write", "patch"],
  planRequired: true,
  planName: "plan.jsonc",
  scopeName: "scope.jsonc",
  skipScopeWhenNoScopeFile: true,
}

function loadConfig(dir: string): KarpathyConfig {
  try {
    const p = join(dir, ".opencode", "karpathy.json")
    if (existsSync(p)) return { ...DEFAULT, ...JSON.parse(readFileSync(p, "utf8")) }
  } catch { /* fall back */ }
  return { ...DEFAULT }
}

function findPlanFile(dir: string, name: string): string | null {
  for (const candidate of [name, `.opencode/${name}`]) {
    const p = join(dir, candidate)
    if (existsSync(p)) return p
  }
  return null
}

function findScopeFile(dir: string, name: string): string | null {
  for (const candidate of [name, `.opencode/${name}`]) {
    const p = join(dir, candidate)
    if (existsSync(p)) return p
  }
  return null
}

function allowedScopes(scopePath: string): string[] {
  try {
    const raw = readFileSync(scopePath, "utf8").trim()
    // Accept JSONC with comments: strip // and /* */ lines
    const clean = raw.replace(/\/\/.*$/gm, "").replace(/\/\*[\s\S]*?\*\//g, "")
    const parsed = JSON.parse(clean)
    if (Array.isArray(parsed)) return parsed
    if (Array.isArray(parsed.scopes)) return parsed.scopes
    if (Array.isArray(parsed.include)) return parsed.include
  } catch { /* invalid scope file */ }
  return []
}

function isInScope(filePath: string, scopes: string[]): boolean {
  if (!scopes.length) return true
  const normalized = filePath.replace(/\\/g, "/")
  return scopes.some((s) => normalized.startsWith(s.replace(/\\/g, "/")))
}

export const KarpathyGuards: Plugin = async ({ directory }) => {
  const cfg = loadConfig(directory)

  return {
    "tool.execute.before": async (input: any) => {
      const tool: string = input?.tool ?? input?.name ?? ""
      if (!cfg.fileMutationTools.includes(tool)) return

      // P1: plan-gate
      if (cfg.planRequired) {
        const planPath = findPlanFile(directory, cfg.planName)
        if (!planPath) {
          throw new Error(
            `[karpathy/P1] No plan file found (${cfg.planName}). Before editing, write a plan (.opencode/${cfg.planName}) ` +
            "with: approach, files to touch, key contract types, and the boundary condition to check. " +
            "One line is fine for trivial work; omit only if the task truly fits in a single edit with no side effects."
          )
        }
      }

      // P3: scope-block
      if (cfg.skipScopeWhenNoScopeFile) {
        const scopePath = findScopeFile(directory, cfg.scopeName)
        if (scopePath) {
          const scopes = allowedScopes(scopePath)
          const path = input?.args?.filePath ?? input?.args?.path ?? input?.args?.file ?? ""
          if (path && !isInScope(path, scopes)) {
            throw new Error(
              `[karpathy/P3] '${path}' is outside the scope defined in ${cfg.scopeName}. ` +
              `Allowed scopes: ${scopes.join(", ") || "(none)"}. Update scope.jsonc if you need to touch this file.`
            )
          }
        }
      }
    },
  }
}
