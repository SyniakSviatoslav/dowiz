import { spawn, ChildProcess } from 'child_process';

const children: ChildProcess[] = [];

function runProcess(name: string, cmd: string, args: string[], env: any): Promise<ChildProcess> {
  return new Promise((resolve) => {
    console.log(`Starting ${name}...`);
    const proc = spawn(cmd, args, { env: { ...process.env, ...env }, shell: true });
    children.push(proc);
    
    // We don't pipe stdout to avoid clutter, only stderr
    proc.stderr?.on('data', (data) => console.error(`[${name} ERR]`, data.toString().trim()));
    
    resolve(proc);
  });
}

async function waitForHealth(url: string, retries = 30) {
  for (let i = 0; i < retries; i++) {
    try {
      const res = await fetch(url);
      if (res.ok) return true;
    } catch {
      // server not ready yet — retry
      console.debug('[verify-stage10] health check failed, retrying');
    }
    await new Promise(r => setTimeout(r, 500));
  }
  throw new Error(`Timeout waiting for ${url}`);
}

async function runTests() {
  console.log('Running tests...');
  return new Promise<void>((resolve, reject) => {
    const proc = spawn('npx', ['tsx', '--env-file=.env', 'attic/apps-api/tests/test-stage10.ts'], { stdio: 'inherit', shell: true });
    proc.on('exit', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`Tests failed with code ${code}`));
    });
  });
}

async function main() {
  try {
    // 1. Start processes
    await runProcess('API-1', 'pnpm', ['--filter', '@deliveryos/api', 'run', 'dev'], { PORT: '3003' });
    await runProcess('API-2', 'pnpm', ['--filter', '@deliveryos/api', 'run', 'dev'], { PORT: '3004' });
    await runProcess('WORKER', 'pnpm', ['--filter', '@deliveryos/worker', 'run', 'dev'], {});

    // 2. Wait for APIs to be healthy
    console.log('Waiting for APIs to be healthy...');
    await waitForHealth('http://127.0.0.1:3003/health');
    await waitForHealth('http://127.0.0.1:3004/health');
    console.log('APIs are healthy!');
    
    // Allow worker a moment to boot
    await new Promise(r => setTimeout(r, 2000));

    // 3. Run tests
    await runTests();
    
    console.log('✅ verify:n2 completed successfully!');
  } catch (err) {
    console.error('❌ verify:n2 failed:', err);
    process.exitCode = 1;
  } finally {
    console.log('Cleaning up processes...');
    for (const proc of children) {
      if (process.platform === 'win32') {
        spawn('taskkill', ['/pid', proc.pid!.toString(), '/f', '/t']);
      } else {
        proc.kill('SIGINT');
      }
    }
  }
}

main();
