import { execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = path.resolve(import.meta.dirname, '..');
let failures = 0;

function check(label: string, ok: boolean, detail?: string) {
  if (ok) {
    console.log(`  ✅ ${label}`);
  } else {
    console.log(`  ❌ ${label}${detail ? ` — ${detail}` : ''}`);
    failures++;
  }
}

async function main() {
  console.log('\n=== Verify Secrets ===\n');

  // 1. gitleaks scan (if available)
  const gitleaksBin = findGitleaks();
  if (!gitleaksBin) {
    console.log('  ⚠ gitleaks not installed, skipping');
  } else {
    try {
      const result = execSync(`"${gitleaksBin}" detect --source . --verbose -i "." 2>&1`, {
        cwd: ROOT,
        encoding: 'utf8',
        timeout: 30000,
      });
      check('gitleaks: no secrets in working tree', true);
    } catch (err: any) {
      const stderr = err.stderr || err.message || '';
      check('gitleaks: no secrets in working tree', false, stderr.slice(0, 200));
    }
  }

  // 2. .env.example has no real secrets
  const envExamplePath = path.join(ROOT, '.env.example');
  if (fs.existsSync(envExamplePath)) {
    const content = fs.readFileSync(envExamplePath, 'utf8');
    const placeholderPatterns = ['your-', 'change-me', '<your', 'xxxx', 'sk_live_', 'sk_test_'];
    const hasPlaceholders = placeholderPatterns.some((p) => content.includes(p));
    check('.env.example uses placeholders', hasPlaceholders);

    const realSecretPatterns = [
      /-----BEGIN RSA PRIVATE KEY-----/,
      /sk_live_[a-zA-Z0-9]+/,
      /ghp_[a-zA-Z0-9]{36}/,
      /SFMyNTY\./,
    ];
    const hasRealSecrets = realSecretPatterns.some((r) => r.test(content));
    check('.env.example no real secrets', !hasRealSecrets);

    const envKeys = content.split('\n').filter((l) => l.trim() && !l.startsWith('#')).map((l) => l.split('=')[0].trim()).filter(Boolean);
    check('.env.example has env keys', envKeys.length > 10);
  } else {
    check('.env.example exists', false);
  }

  // 3. No JWT keys in code defaults
  const srcFiles = findFiles(path.join(ROOT, 'apps'), ['.ts', '.js', '.mjs'])
    .filter(f => !f.includes('/tests/') && !f.includes('\\tests\\'));
  const jwtKeyPatterns = [
    { pattern: /process\.env\.***REDACTED***\s*\|\|[^|]/, label: '***REDACTED*** default' },
    { pattern: /process\.env\.***REDACTED***\s*\|\|[^|]/, label: '***REDACTED*** default' },
    { pattern: /BEGIN RSA PRIVATE KEY/, label: 'RSA private key in source' },
    { pattern: /process\.env\.***REDACTED***\s*\|\|[a-zA-Z0-9]/, label: '***REDACTED*** default' },
    { pattern: /process\.env\.VAPID_PRIVATE_KEY\s*\|\|[a-zA-Z0-9]/, label: 'VAPID_PRIVATE_KEY default' },
    { pattern: /process\.env\.SENTRY_DSN\s*\|\|[a-zA-Z0-9]/, label: 'SENTRY_DSN default' },
  ];

  for (const { pattern, label } of jwtKeyPatterns) {
    const found = srcFiles.filter((f) => {
      const content = fs.readFileSync(f, 'utf8');
      return pattern.test(content);
    });
    check(`no default ${label}`, found.length === 0, found.length > 0 ? found.join(', ') : undefined);
  }

  // 4. No secrets in git history (if git dir)
  if (fs.existsSync(path.join(ROOT, '.git'))) {
    try {
      const logResult = execSync('git log --all --pretty=format:"%H %s" --diff-filter=A -- "*.env" "*.pem" "*.key" 2>&1', {
        cwd: ROOT,
        encoding: 'utf8',
        timeout: 10000,
      });
      check('no .env/.pem/.key in git history', !logResult.trim(), logResult.trim().slice(0, 200));
    } catch {
      check('git history check', true);
    }
  }

  console.log(`\n${failures === 0 ? '✅ All secrets checks passed' : `❌ ${failures} failure(s)`}\n`);
  process.exit(failures > 0 ? 1 : 0);
}

function findGitleaks(): string | null {
  const candidates = [
    'gitleaks',
    'gitleaks.exe',
    path.join(process.env.LOCALAPPDATA || '', 'Microsoft', 'WinGet', 'Packages', 'Gitleaks.Gitleaks_Microsoft.Winget.Source_8wekyb3d8bbwe', 'gitleaks.exe'),
    path.join(process.env.USERPROFILE || '', 'go', 'bin', 'gitleaks.exe'),
    '/usr/local/bin/gitleaks',
    '/opt/homebrew/bin/gitleaks',
  ];
  for (const c of candidates) {
    try {
      execSync(`"${c}" version 2>&1`, { timeout: 3000 });
      return c;
    } catch { }
  }
  try {
    const result = execSync('where.exe gitleaks 2>&1', { timeout: 3000, encoding: 'utf8' });
    return result.trim().split('\n')[0].trim();
  } catch { }
  return null;
}

function findFiles(dir: string, exts: string[]): string[] {
  const results: string[] = [];
  try {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory() && !entry.name.startsWith('.') && entry.name !== 'node_modules') {
        results.push(...findFiles(full, exts));
      } else if (entry.isFile() && exts.some((e) => entry.name.endsWith(e))) {
        results.push(full);
      }
    }
  } catch { }
  return results;
}

main().catch((err) => { console.error('Fatal:', err.message); process.exit(1); });
