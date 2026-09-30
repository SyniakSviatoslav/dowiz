// Box network: TCP connect to Cloudflare takes 4-11 s and undici gives up at 10 s.
// A request whose CONNECT timed out never reached the server, so retrying it is safe
// even for a POST. Anything else (a server answer, a reset mid-request) is not retried.
const real = globalThis.fetch;
globalThis.fetch = async (...a) => {
  for (let i = 0; ; i++) {
    try { return await real(...a); }
    catch (e) {
      if (i < 5 && e?.cause?.code === 'UND_ERR_CONNECT_TIMEOUT') { process.stderr.write(`[retry-fetch] connect timeout, retry ${i + 1}\n`); continue; }
      throw e;
    }
  }
};
