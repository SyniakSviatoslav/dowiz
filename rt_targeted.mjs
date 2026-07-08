// Targeted verification of specific red-team concerns.
import { spawn } from 'node:child_process';

const REPO = '/root/bebop-repo';
const MEM = '/tmp/rt3/mem.json';

function run(name, setup) {
  return new Promise((resolve) => {
    const child = spawn('npx', ['tsx', 'bebop.ts', 'mcp'], {
      cwd: REPO, env: { ...process.env, BEBOP_MEMORY_PATH: MEM },
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    let errOut = '', out = '', exitInfo = null, uncaught = null;
    child.stdout.on('data', (c) => { out += c; });
    child.stderr.on('data', (c) => { errOut += c; });
    child.on('uncaughtException', (e) => { uncaught = e.message; }); // not standard but try
    child.on('exit', (code, sig) => { exitInfo = { code, sig }; });

    const done = (note) => {
      if (out.trim()) out = out.trim().slice(0, 200);
      resolve({ name, note, exit: exitInfo, errTail: errOut.replace(/\n/g, ' ').slice(0, 300), out, uncaught });
    };

    setup(child, done);
  });
}

async function main() {
  const res = [];

  // (A) null request line — does it crash (unhandled TypeError on null.id) or silently drop?
  res.push(await run('A: line "null"', (child, done) => {
    child.stdin.write('null\n');
    setTimeout(() => { try { child.stdin.end(); } catch {} }, 300);
    setTimeout(() => done('after close'), 4000);
  }));

  // (B) batch array of 3 — spec: must return an array response or single error. Observe output.
  res.push(await run('B: batch array [init,tools/list,boot]', (child, done) => {
    child.stdin.write('[' +
      '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}},' +
      '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}},' +
      '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"bebop_boot","arguments":{}}}' +
      ']\n');
    setTimeout(() => { try { child.stdin.end(); } catch {} }, 300);
    setTimeout(() => done('after close'), 4000);
  }));

  // (C) EPIPE: client closes its read end (parent destroys stdout) while server writes.
  res.push(await run('C: EPIPE (parent destroys stdout read)', (child, done) => {
    // send a big tools/list then immediately destroy OUR read side so server write fails
    child.stdin.write('{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}\n');
    setTimeout(() => {
      try { child.stdout.destroy(); } catch {}
    }, 50);
    setTimeout(() => { try { child.stdin.end(); } catch {} }, 100);
    setTimeout(() => done('after timeout'), 5000);
  }));

  // (D) EPIPE heavy: send many requests then destroy stdout — forces a write after pipe closed.
  res.push(await run('D: EPIPE heavy (100 reqs + destroy stdout)', (child, done) => {
    let s = '';
    for (let i = 0; i < 100; i++) s += '{"jsonrpc":"2.0","id":' + i + ',"method":"tools/list","params":{}}\n';
    child.stdin.write(s);
    setTimeout(() => { try { child.stdout.destroy(); } catch {} }, 30);
    setTimeout(() => done('after timeout'), 5000);
  }));

  // (E) invalid request object as array-element with invalid json inside batch? Already B.
  // (F) missing id that is NOT a known notification -> should this get an error response?
  res.push(await run('F: missing id, unknown method', (child, done) => {
    child.stdin.write('{"jsonrpc":"2.0","method":"frobnicate","params":{}}\n');
    setTimeout(() => { try { child.stdin.end(); } catch {} }, 300);
    setTimeout(() => done('after close'), 4000);
  }));

  console.log('\n================ TARGETED FINDINGS ================');
  for (const r of res) {
    console.log('### ' + r.name);
    console.log('   note   : ' + r.note);
    console.log('   exit   : ' + JSON.stringify(r.exit) + ' uncaught=' + r.uncaught);
    console.log('   stdout : ' + JSON.stringify(r.out).slice(0, 200));
    console.log('   stderr : ' + r.errTail);
  }
  console.log('==================================================');
}
main().catch((e) => { console.error('ERR', e); process.exit(1); });
