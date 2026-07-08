// Red-team harness for the bebop MCP stdio JSON-RPC server.
import { spawn } from 'node:child_process';

const REPO = '/root/bebop-repo';
const MEM = '/tmp/rt3/mem.json';
const log = (...a) => console.log(...a);

function runCase(name, lines, opts = {}) {
  return new Promise((resolve) => {
    const child = spawn('npx', ['tsx', 'bebop.ts', 'mcp'], {
      cwd: REPO,
      env: { ...process.env, BEBOP_MEMORY_PATH: MEM },
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    let out = '';
    let errOut = '';
    let crashed = false;
    let exitCode = null;
    let exitSignal = null;
    const nonJson = [];
    const responses = [];

    child.stdout.setEncoding('utf8');
    child.stdout.on('data', (chunk) => {
      out += chunk;
      const parts = out.split('\n');
      out = parts.pop();
      for (const p of parts) {
        if (!p.trim()) continue;
        try { responses.push(JSON.parse(p)); } catch { nonJson.push(p); }
      }
    });
    child.stderr.setEncoding('utf8');
    child.stderr.on('data', (c) => { errOut += c; });
    child.on('error', (e) => { crashed = true; errOut += '\n[spawn error] ' + e.message; });

    const input = lines.join('\n') + '\n';
    try { child.stdin.write(input); } catch (e) { errOut += '\n[stdin write error] ' + e.message; }

    const finalize = () => {
      if (out.trim()) {
        for (const p of out.split('\n')) {
          if (!p.trim()) continue;
          try { responses.push(JSON.parse(p)); } catch { nonJson.push(p); }
        }
      }
      let outcome;
      if (crashed) outcome = 'CRASH (spawn/exit error)';
      else if (exitSignal === 'SIGKILL' && opts.hardKill) outcome = 'HANG (killed by timeout, no exit)';
      else if (exitCode !== null && exitCode !== 0) outcome = 'CRASH (exit code ' + exitCode + (exitSignal ? ' sig ' + exitSignal : '') + ')';
      else if (nonJson.length) outcome = 'NON-JSON emitted: ' + nonJson.slice(0, 2).join(' | ');
      else if (responses.length === 0) outcome = 'NO RESPONSE (exited 0, expected output)';
      else outcome = 'OK (exit ' + exitCode + ', ' + responses.length + ' responses)';
      resolve({ name, outcome, nResp: responses.length, nonJson: nonJson.length, errOut: errOut.slice(0, 400), exitCode, exitSignal, crashed });
    };

    child.on('exit', (code, sig) => { exitCode = code; exitSignal = sig; finalize(); });

    // close stdin so the server exits naturally (it waits on 'end')
    const closeDelay = opts.closeDelay ?? 200;
    setTimeout(() => { try { child.stdin.end(); } catch {} }, closeDelay);

    // hard timeout => HANG
    const timeout = opts.timeout ?? 10000;
    setTimeout(() => {
      if (exitCode === null) { opts.hardKill = true; try { child.kill('SIGKILL'); } catch {} }
    }, timeout);

    // killEarly: simulate client vanishing immediately after sending (tests EPIPE / unhandled write)
    if (opts.killEarly) {
      setTimeout(() => { try { child.kill('SIGKILL'); } catch {} }, 50);
    }
  });
}

const HUGE = 'A'.repeat(10 * 1024 * 1024);
const NEST_BOMB = JSON.stringify((() => { let o = {}; for (let i = 0; i < 14; i++) o = { a: o }; return o; })());

async function main() {
  const results = [];
  results.push(await runCase('baseline: initialize+tools/list+bebop_boot',
    ['{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}',
     '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}',
     '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"bebop_boot","arguments":{}}}']));

  results.push(await runCase('malformed: unclosed brace', ['{"jsonrpc":"2.0","id":4,"method":"initialize"']));
  results.push(await runCase('malformed: trailing garbage', ['{"jsonrpc":"2.0","id":5,"method":"initialize"}\ngarbage not json\n']));
  results.push(await runCase('malformed: not JSON', ['hello world this is not json']));
  results.push(await runCase('malformed: truncated mid-string', ['{"jsonrpc":"2.0","id":6,"method":"ini']));

  results.push(await runCase('unknown method', ['{"jsonrpc":"2.0","id":7,"method":"frobnicate","params":{}}']));

  results.push(await runCase('missing jsonrpc field', ['{"id":8,"method":"tools/list","params":{}}']));
  results.push(await runCase('missing id (notification-style)', ['{"jsonrpc":"2.0","method":"tools/list","params":{}}']));
  results.push(await runCase('empty object', ['{}']));
  results.push(await runCase('null request', ['null']));
  results.push(await runCase('number request', ['42']));

  results.push(await runCase('huge string param (5MB)', ['{"jsonrpc":"2.0","id":9,"method":"tools/list","params":{"x":"' + 'B'.repeat(5 * 1024 * 1024) + '"}}']));
  results.push(await runCase('nested bomb param', ['{"jsonrpc":"2.0","id":10,"method":"tools/list","params":' + NEST_BOMB + '}']));
  results.push(await runCase('params is array', ['{"jsonrpc":"2.0","id":11,"method":"tools/call","params":[1,2,3]}']));

  results.push(await runCase('remember payload=null', ['{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"bebop_remember","arguments":{"concept":"x","payload":null}}']));
  results.push(await runCase('remember 10MB payload', ['{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"bebop_remember","arguments":{"concept":"x","payload":"' + HUGE + '"}}'], { timeout: 60000, closeDelay: 1500 }));
  results.push(await runCase('remember control chars payload', ['{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"bebop_remember","arguments":{"concept":"x","payload":"\u0000\u0007\u0008\u000b\u000c"}}']));
  results.push(await runCase('remember oversized concept (10MB)', ['{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"bebop_remember","arguments":{"concept":"' + HUGE + '","payload":"p"}}'], { timeout: 60000, closeDelay: 1500 }));
  results.push(await runCase('remember array payload', ['{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{"name":"bebop_remember","arguments":{"concept":"x","payload":[1,2,3]}}']));
  results.push(await runCase('remember missing arguments', ['{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"bebop_remember"}}']));

  results.push(await runCase('recall no query', ['{"jsonrpc":"2.0","id":18,"method":"tools/call","params":{"name":"bebop_recall","arguments":{}}']));
  results.push(await runCase('recall query null', ['{"jsonrpc":"2.0","id":19,"method":"tools/call","params":{"name":"bebop_recall","arguments":{"query":null}}']));
  results.push(await runCase('recall 10MB query', ['{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"bebop_recall","arguments":{"query":"' + HUGE + '"}}'], { timeout: 60000, closeDelay: 1500 }));

  const batch = [];
  for (let i = 0; i < 1000; i++) batch.push('{"jsonrpc":"2.0","id":' + (1000 + i) + ',"method":"tools/list","params":{}}');
  results.push(await runCase('batch 1000 separate lines', batch, { timeout: 30000, closeDelay: 500 }));

  const batchArr = '[' + batch.slice(0, 50).join(',') + ']';
  results.push(await runCase('JSON-RPC batch array (50)', [batchArr], { timeout: 15000 }));

  // disconnect mid-response: spawn, send one big request, kill the child's stdout reader by SIGKILL of the child after partial?
  // Instead: test EPIPE by writing then immediately SIGKILL the child (simulates client vanish).
  results.push(await runCase('abrupt client death after send', ['{"jsonrpc":"2.0","id":2001,"method":"tools/list","params":{}}'], { timeout: 3000, hardKill: true, killEarly: true }));

  log('\n==================== RED-TEAM REPORT ====================');
  let weak = 0;
  for (const r of results) {
    log('### ' + r.name);
    log('   outcome: ' + r.outcome + ' | resp=' + r.nResp + ' nonJson=' + r.nonJson + ' exit=' + r.exitCode + ' sig=' + r.exitSignal + ' crashed=' + r.crashed);
    if (r.errOut) log('   stderr: ' + r.errOut.replace(/\n/g, ' ').slice(0, 250));
    if (/CRASH|HANG|NON-JSON|NO RESPONSE/.test(r.outcome)) weak++;
  }
  log('========================================================');
  if (weak === 0) log('MCP ROBUST — no crash/hang/non-JSON across all fuzz cases.');
  else { log('WEAKNESSES FOUND: ' + weak); for (const r of results) if (/CRASH|HANG|NON-JSON|NO RESPONSE/.test(r.outcome)) log('  WEAKNESS: ' + r.name + ' -> ' + r.outcome); }
}

main().catch((e) => { console.error('HARNESS ERROR', e); process.exit(1); });
