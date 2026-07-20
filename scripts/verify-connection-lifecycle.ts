import { readFileSync, statSync, globSync } from 'node:fs';
import { resolve } from 'node:path';

const ROOT = process.cwd();

function findCalls(filePath: string, method: string): { line: number; match: string }[] {
  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const results: { line: number; match: string }[] = [];
  const regex = new RegExp(`\\.${method}\\s*\\(`, 'g');
  for (let i = 0; i < lines.length; i++) {
    if (regex.test(lines[i])) {
      results.push({ line: i + 1, match: lines[i].trim().substring(0, 80) });
    }
  }
  return results;
}

function main() {
  console.log('\n=== Connection Lifecycle Audit ===\n');

  const searchDirs = [
    resolve(ROOT, 'apps/api/src'),
    resolve(ROOT, 'packages/platform/src'),
  ];
  try {
    const workerDir = resolve(ROOT, 'apps/worker/src');
    if (statSync(workerDir).isDirectory()) searchDirs.push(workerDir);
  } catch {}

  const tsFiles: string[] = [];
  for (const dir of searchDirs) {
    try {
      const files = globSync(`${dir}/**/*.ts`, { nodir: true });
      tsFiles.push(...files);
    } catch {}
  }

  console.log(`Scanning ${tsFiles.length} TypeScript files for connection patterns...\n`);

  let totalConnects = 0;
  let totalCloses = 0;
  const unpaired: { file: string; line: number; match: string }[] = [];

  for (const file of tsFiles) {
    const relativePath = file.replace(ROOT, '').replace(/\\/g, '/');
    const content = readFileSync(file, 'utf-8');
    const lines = content.split('\n');

    const connects = findCalls(file, 'connect');
    const closes = findCalls(file, 'release');
    const ends = findCalls(file, 'end');
    const allCloses = [...closes, ...ends];

    for (const conn of connects) {
      totalConnects++;
      const scopeEnd = Math.min(conn.line + 50, lines.length);
      const hasClose = allCloses.some(c => c.line > conn.line && c.line <= scopeEnd);

      if (!hasClose) {
        const classClose = allCloses.find(c => c.line > conn.line);
        if (!classClose) {
          unpaired.push({ file: relativePath, line: conn.line, match: conn.match });
        }
      } else {
        totalCloses++;
      }
    }
  }

  console.log(`Total .connect() calls:  ${totalConnects}`);
  console.log(`Total .release()/.close()/.end(): ${totalCloses}`);

  if (unpaired.length > 0) {
    console.warn(`\n⚠️  Potential unpaired connect() calls (${unpaired.length}):`);
    for (const item of unpaired.slice(0, 15)) {
      console.warn(`   ${item.file}:${item.line} — ${item.match}`);
    }
    if (unpaired.length > 15) {
      console.warn(`   ... and ${unpaired.length - 15} more`);
    }
    console.warn(`\nNote: Some of these may be class-level pools or shared connections.`);
    console.warn(`Manual review needed for each flagged location.`);
  } else {
    console.log(`\n✅ No unpaired connect() calls found.`);
  }

  console.log(`\n=== Summary ===`);
  if (unpaired.length === 0) {
    console.log(`✅ Connection lifecycle looks clean.`);
    process.exit(0);
  } else {
    console.log(`⚠️  ${unpaired.length} potential issues flagged — review required.`);
    process.exit(0);
  }
}

main().catch(err => {
  console.error('Connection lifecycle audit failed:', err);
  process.exit(1);
});
