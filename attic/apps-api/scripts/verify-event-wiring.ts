import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROVDER_PATH = resolve(__dirname, '../src/notifications/provider.ts');
const LOCALES_PATH = resolve(__dirname, '../src/notifications/locales.ts');
const RENDER_PATH = resolve(__dirname, '../src/notifications/render.ts');
const WORKERS_PATH = resolve(__dirname, '../src/notifications/workers/index.ts');

function extractEventTypes(filePath: string): string[] {
  const content = readFileSync(filePath, 'utf-8');
  const typeMatch = content.match(/export type NotificationEventType\s*=\s*([\s\S]+?)(?=;)/);
  if (!typeMatch) throw new Error('Could not find NotificationEventType union');
  const unionBlock = typeMatch[1];
  const types: string[] = [];
  const regex = /\|?\s*'([^']+)'/g;
  let match;
  while ((match = regex.exec(unionBlock)) !== null) {
    types.push(match[1]);
  }
  return types.sort();
}

function extractLocaleKeys(filePath: string): string[] {
  const content = readFileSync(filePath, 'utf-8');
  const keys = new Set<string>();
  const regex = /'([^']+)':\s*\(v\)/g;
  let match;
  while ((match = regex.exec(content)) !== null) {
    keys.add(match[1]);
  }
  return [...keys].sort();
}

function extractRenderCases(filePath: string): string[] {
  const content = readFileSync(filePath, 'utf-8');
  const cases = new Set<string>();
  const regex = /case\s+'([^']+)':/g;
  let match;
  while ((match = regex.exec(content)) !== null) {
    cases.add(match[1]);
  }
  return [...cases].sort();
}

function extractBuildTelegramDataCases(filePath: string): string[] {
  const content = readFileSync(filePath, 'utf-8');
  const cases = new Set<string>();
  const regex = /case\s+'([^']+)':/g;
  let match;
  while ((match = regex.exec(content)) !== null) {
    cases.add(match[1]);
  }
  return [...cases].sort();
}

function main() {
  console.log('\n=== Event Wiring Completeness Check ===\n');

  const expected = extractEventTypes(PROVDER_PATH);
  console.log(`Defined event types: ${expected.length}`);
  console.log(`  ${expected.join('\n  ')}\n`);

  // Locales — check sq
  const sqKeys = extractLocaleKeys(LOCALES_PATH);
  const sqMissing = expected.filter(e => !sqKeys.includes(e));
  console.log(`Locale 'sq' keys: ${sqKeys.length}`);
  if (sqMissing.length > 0) {
    console.log(`  ❌ MISSING: ${sqMissing.join(', ')}`);
  } else {
    console.log(`  ✅ All events have 'sq' locale`);
  }

  // Locales — check en
  const enKeys = extractLocaleKeys(LOCALES_PATH);
  const enMissing = expected.filter(e => !enKeys.includes(e));
  console.log(`Locale 'en' keys: ${enKeys.length}`);
  if (enMissing.length > 0) {
    console.log(`  ❌ MISSING: ${enMissing.join(', ')}`);
  } else {
    console.log(`  ✅ All events have 'en' locale`);
  }

  // Locales — check uk
  const ukKeys = extractLocaleKeys(LOCALES_PATH);
  const ukMissing = expected.filter(e => !ukKeys.includes(e));
  console.log(`Locale 'uk' keys: ${ukKeys.length}`);
  if (ukMissing.length > 0) {
    console.log(`  ❌ MISSING: ${ukMissing.join(', ')}`);
  } else {
    console.log(`  ✅ All events have 'uk' locale`);
  }

  // Render switch cases
  const renderCases = extractRenderCases(RENDER_PATH);
  const renderMissing = expected.filter(e => !renderCases.includes(e));
  console.log(`\nRender switch cases: ${renderCases.length}`);
  if (renderMissing.length > 0) {
    console.log(`  ❌ MISSING: ${renderMissing.join(', ')}`);
  } else {
    console.log(`  ✅ All events handled in render.ts`);
  }

  // BuildTelegramData switch cases
  const workerCases = extractBuildTelegramDataCases(WORKERS_PATH);
  const workerMissing = expected.filter(e => !workerCases.includes(e));
  console.log(`buildTelegramData switch cases: ${workerCases.length}`);
  if (workerMissing.length > 0) {
    console.log(`  ❌ MISSING: ${workerMissing.join(', ')}`);
  } else {
    console.log(`  ✅ All events handled in workers/index.ts buildTelegramData`);
  }

  // Summary
  const allMissing = [...new Set([...sqMissing, ...enMissing, ...ukMissing, ...renderMissing, ...workerMissing])];
  console.log(`\n=== VERDICT ===`);
  if (allMissing.length === 0) {
    console.log(`✅ All ${expected.length} event types are fully wired through the notification chain.`);
    process.exit(0);
  } else {
    console.error(`❌ ${allMissing.length} event type(s) have wiring gaps:`);
    for (const event of allMissing.sort()) {
      const gaps: string[] = [];
      if (sqMissing.includes(event)) gaps.push('sq locale');
      if (enMissing.includes(event)) gaps.push('en locale');
      if (ukMissing.includes(event)) gaps.push('uk locale');
      if (renderMissing.includes(event)) gaps.push('render.ts case');
      if (workerMissing.includes(event)) gaps.push('workers/index.ts case');
      console.error(`   - '${event}': ${gaps.join(', ')}`);
    }
    process.exit(1);
  }
}

main();
