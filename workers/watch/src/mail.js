// The alert: one plain-text mail per tick that has transitions, through Email Routing's send_email
// binding to the one verified destination. Header values are cleaned of CR/LF so a target name or
// a body snippet can never add a header (the waitlist's mail_raw learned that the hard way).

const clean = (s) => String(s == null ? '' : s).replace(/[\r\n]+/g, ' ').slice(0, 300);

/// The raw RFC 5322 message. Pure; `id` makes the Message-ID unique per tick.
export function mailRaw({ from, to, changes, atMs, statusUrl, id }) {
  const down = changes.filter((c) => c.to !== 'up').length;
  const up = changes.length - down;
  const subject = clean(`dowiz-watch: ${down ? `${down} DOWN` : ''}${down && up ? ', ' : ''}${up ? `${up} back up` : ''}`);
  const lines = changes.map((c) => `${c.to.toUpperCase()}  ${clean(c.name)}  (was ${c.from || 'unknown'})  ${clean(c.detail)}  ${clean(c.url)}`);
  const body = [
    `Checked at ${new Date(atMs).toISOString()} from inside Cloudflare.`,
    '',
    ...lines,
    '',
    `Full state: ${clean(statusUrl)}`,
    '',
  ].join('\r\n');
  return [
    `From: dowiz-watch <${clean(from)}>`,
    `To: <${clean(to)}>`,
    `Subject: ${subject}`,
    `Message-ID: <${clean(id)}@dowiz.org>`,
    `Date: ${new Date(atMs).toUTCString()}`,
    'MIME-Version: 1.0',
    'Content-Type: text/plain; charset=utf-8',
    '',
    body,
  ].join('\r\n');
}

/// Send it. Returns {ok, error?}: never throws, because a mail that failed is a fact for /status,
/// not a reason to lose the tick.
export async function send(env, raw) {
  if (!env.ALERT_MAIL) return { ok: false, error: 'no ALERT_MAIL binding' };
  try {
    const { EmailMessage } = await import('cloudflare:email');
    await env.ALERT_MAIL.send(new EmailMessage(env.MAIL_FROM, env.MAIL_TO, raw));
    return { ok: true };
  } catch (e) {
    return { ok: false, error: String((e && e.message) || e).slice(0, 200) };
  }
}
