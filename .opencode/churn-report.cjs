// DeliveryOS harness — Karpathy P2 churn report (Simplicity).
// Run: node .opencode/churn-report.cjs [dir=.] [fileCount=10]
// Uses only git diff --stat, no external deps.
// Reports: files with most edits in last N commits, + big files.

const { execSync } = require("child_process")
const { existsSync, readFileSync } = require("fs")

const dir = process.argv[2] || "."
const limit = parseInt(process.argv[3], 10) || 10
const maxCommits = process.argv[4] || "100"

function sh(cmd) {
  try {
    return execSync(cmd, { cwd: dir, encoding: "utf8", maxBuffer: 10 * 1024 * 1024 })
  } catch {
    return ""
  }
}

const delim = "|"
const header = `${"FILE".padEnd(80)}${delim}CHANGES${delim}CHURN%${delim}LINES`

console.log(`\n=== Churn Report (last ${maxCommits} commits) ===\n`)
console.log(header)
console.log("-".repeat(header.length))

const log = sh(`git log --oneline -${maxCommits} --diff-filter=M --name-only --pretty=format:"H %H" "${dir}"`)
const fileCounts = {}
const fileSet = new Set()
let currentCommit = ""
for (const line of log.split("\n")) {
  const m = line.match(/^H (.+)/)
  if (m) {
    currentCommit = m[1]
  } else if (line.trim() && currentCommit) {
    fileSet.add(line.trim())
    fileCounts[line.trim()] = (fileCounts[line.trim()] || 0) + 1
  }
}

const totCommits = log.split("\n").filter(l => l.startsWith("H ")).length || 1
const sorted = Object.entries(fileCounts)
  .sort((a, b) => b[1] - a[1])
  .slice(0, limit)

for (const [file, count] of sorted) {
  let lines = "?"
  try {
    if (existsSync(file)) {
      const content = readFileSync(file, "utf8")
      lines = String(content.split("\n").length)
    }
  } catch { lines = "?" }
  const pct = ((count / totCommits) * 100).toFixed(1)
  console.log(`${file.padEnd(80)}${delim}${String(count).padStart(3)}${delim}${pct.padStart(5)}%${delim}${lines.padStart(5)}`)
}

console.log(`\n=== Biggest files (top ${limit} by line count) ===\n`)
const bigFiles = sh(`git ls-files "${dir}"`)
  .split("\n")
  .filter(Boolean)
  .map(f => {
    try {
      if (!existsSync(f)) return null
      const content = readFileSync(f, "utf8")
      return { file: f, lines: content.split("\n").length }
    } catch { return null }
  })
  .filter(Boolean)
  .sort((a, b) => b.lines - a.lines)
  .slice(0, limit)

for (const f of bigFiles) {
  console.log(`${f.file.padEnd(80)}${delim}${String(f.lines).padStart(6)} lines`)
}

console.log("\n(P2: files above 500 lines are churn risk. Files with >20% churn rate over 100 commits are volatile.)\n")
