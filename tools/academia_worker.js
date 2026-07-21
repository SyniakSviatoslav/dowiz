// academia_worker.js — Lightweight relay + status.
// Академія Дмитра Євдокимова
// Важка робота: academia_seed server + HF CDN.
// Worker: координація + статус.

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    
    // Status — миттєво
    if (url.pathname === '/status') {
      let r2info = {};
      try {
        if (env?.ACADEMIA_BUCKET) {
          const objs = await env.ACADEMIA_BUCKET.list({ limit: 10 });
          r2info = { chunks: objs.objects.length, total_size: objs.objects.reduce((a, o) => a + o.size, 0) };
        }
      } catch(e) {}
      
      return new Response(JSON.stringify({
        status: 'active',
        worker: 'academia-mesh',
        note: 'Академія Дмитра Євдокимова',
        r2: r2info,
        hf: 'https://huggingface.co/datasets/Delulu12/academia-matrix',
        seed: 'systemctl status academia-seed',
      }), { headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' }});
    }

    // Stats endpoint
    if (url.pathname === '/stats') {
      let total = 0, chunks = 0;
      try {
        if (env?.ACADEMIA_BUCKET) {
          const objs = await env.ACADEMIA_BUCKET.list();
          chunks = objs.objects.length;
          for (const o of objs.objects) total += parseInt(o.customMetadata?.count || '0');
        }
      } catch(e) {}
      return new Response(JSON.stringify({ total_papers: total, chunks, seed_active: true }),
        { headers: { 'Content-Type': 'application/json' }});
    }

    // Health
    return new Response(JSON.stringify({ ok: true, id: url.searchParams.get('id') || '0' }),
      { headers: { 'Content-Type': 'application/json' }});
  },

  async scheduled(event, env) {
    // Cron: trigger seed server or GitHub Actions
    console.log('Academia cron triggered');
    // The actual extraction is handled by:
    // 1. academia-seed systemd service (this machine)
    // 2. GitHub Actions cron (.github/workflows/)
    // 3. Manual: curl academia_bot.sh | bash
  }
};
