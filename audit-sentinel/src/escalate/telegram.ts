export async function sendTelegramAlert(
  botToken: string,
  chatId: string,
  message: string,
): Promise<boolean> {
  try {
    const url = `https://api.telegram.org/bot${botToken}/sendMessage`;
    const body = JSON.stringify({
      chat_id: chatId,
      text: message,
      parse_mode: 'HTML',
      disable_web_page_preview: true,
    });

    const resp = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
      signal: AbortSignal.timeout(10000),
    });

    const data = await resp.json();
    return data.ok === true;
  } catch (err) {
    console.error('[lint] ignored catch', err);
    console.error('Telegram alert failed:', err);
    return false;
  }
}

export function formatBlockersForTelegram(
  env: string,
  baseUrl: string,
  runId: string,
  blockers: Array<{ id: string; layer: string; target: string; detail: string }>,
): string {
  const lines = [
    `🔴 <b>[NEW BLOCKER · ${env.toUpperCase()}]</b>`,
    `<code>${baseUrl}</code>`,
    `Run: <code>${runId}</code>`,
    ``,
  ];

  for (const b of blockers) {
    lines.push(`• <b>[${b.layer}]</b> ${b.target}: ${b.detail}`);
  }

  lines.push(``);
  lines.push(`Total: ${blockers.length} blocker(s)`);

  return lines.join('\n');
}

export function formatDailigest(
  summary: {
    total: number;
    green: number;
    red: number;
    flaky: number;
    verdict: string;
  },
): string {
  return [
    `📋 <b>Daily Audit Digest</b>`,
    `Verdict: <b>${summary.verdict}</b>`,
    `Checks: ${summary.green}✅ ${summary.red}🔴 ${summary.flaky}⚠️ / ${summary.total}`,
  ].join('\n');
}

const lastAlertTime: Record<string, number> = {};

export function shouldSendAlert(findingId: string, cooldownMs = 24 * 60 * 60 * 1000): boolean {
  const last = lastAlertTime[findingId] || 0;
  const now = Date.now();
  if (now - last < cooldownMs) return false;
  lastAlertTime[findingId] = now;
  return true;
}
