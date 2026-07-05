#!/usr/bin/env node
// Install the distilled project memory (docs/adr/00-project-memory.adr.md) into the
// codebase-memory graph as the persistent project ADR — so every agent that queries the
// graph gets the hard-won memory (the "memories from grep") in one call, not by grepping
// the markdown vault. Re-run after editing the source ADR or after a full re-index.
//   node scripts/codebase-memory/install-memory.mjs
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
const BIN = path.join(os.homedir(), '.local/bin/codebase-memory-mcp');
const SRC = path.resolve('docs/adr/00-project-memory.adr.md');
const content = fs.readFileSync(SRC, 'utf8');
const args = JSON.stringify({ project: 'root-dowiz', mode: 'update', content });
const out = execFileSync(BIN, ['cli', 'manage_adr', args], { encoding: 'utf8' })
  .split('\n').filter((l) => !l.startsWith('level=')).join('\n');
console.log('installed project memory into codebase-memory graph:', out.trim());
