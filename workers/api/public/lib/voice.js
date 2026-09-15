// Speech in, commands out — the browser half of the voice surface.
//
// RECOGNITION HAPPENS HERE, not on the hub. The Web Speech API is already on
// these phones, already speaks the user's language, and costs nothing. Shipping
// audio to the server would need a model, a dependency, bandwidth on a phone at
// the edge of coverage, and would turn "what did the courier say" into a
// question someone could answer from a disk. The hub receives text.
//
// THE HUB DECIDES WHAT THE TEXT MEANT. Nothing here interprets a command: this
// module hears, and posts. That keeps one grammar, in one language-agnostic
// place, reachable identically from every surface — which is what "control the
// service by voice regardless of role and surface" has to mean if it is to be
// more than three separate half-implementations.
//
// IT IS ABSENT, NOT BROKEN, WHERE UNSUPPORTED. Firefox has no
// SpeechRecognition. `create()` returns null there and the caller shows no
// microphone at all, rather than a button that does nothing.

const SR = globalThis.SpeechRecognition || globalThis.webkitSpeechRecognition;

export const supported = () => Boolean(SR);

/// Create a recogniser, or null where the browser has none.
///
/// `lang` is a BCP-47 tag ('uk-UA', 'sq-AL', 'en-GB'). Passing the wrong one is
/// not a small error: a Ukrainian phrase recognised as English comes back as
/// confident nonsense, which is precisely what the hub's confidence floor
/// cannot catch.
export function create({ lang = 'uk-UA', onResult, onError, onEnd } = {}) {
  if (!SR) return null;
  const r = new SR();
  r.lang = lang;
  // Interim results are shown to the speaker so they can see it is listening,
  // but they are marked and the hub refuses to act on them.
  r.interimResults = true;
  r.continuous = false;
  r.maxAlternatives = 1;

  r.onresult = ev => {
    const res = ev.results[ev.results.length - 1];
    const alt = res[0];
    onResult?.({
      transcript: alt.transcript,
      // Chrome reports 0 confidence for interim results and sometimes for final
      // ones on some locales. Treating a missing score as 0 would make the hub
      // refuse every command; treating it as 1 would remove the floor entirely.
      // A middling value keeps the floor meaningful while not being fatal.
      confidence: Number.isFinite(alt.confidence) && alt.confidence > 0 ? alt.confidence : 0.75,
      isFinal: res.isFinal,
    });
  };
  r.onerror = ev => {
    // `no-speech` and `aborted` are ordinary: someone pressed the button and
    // said nothing, or pressed it again. They are not failures to report.
    if (ev.error === 'no-speech' || ev.error === 'aborted') return onEnd?.();
    onError?.(ev.error === 'not-allowed'
      ? 'microphone-denied'
      : ev.error === 'network' ? 'network' : ev.error);
  };
  r.onend = () => onEnd?.();
  return r;
}

/// Read a line back out loud.
///
/// The read-back is what makes a spoken confirmation safe: a person agreeing to
/// something they were only shown has to be looking at the screen, which is
/// exactly the situation voice exists to avoid. Silent where unsupported, and
/// silent when the page is hidden — a phone in a pocket must not start talking.
export function speak(text, lang = 'uk-UA') {
  try {
    if (!globalThis.speechSynthesis || document.hidden || !text) return;
    const u = new SpeechSynthesisUtterance(text);
    u.lang = lang;
    u.rate = 1.05;
    speechSynthesis.cancel();      // never queue: the newest line is the true one
    speechSynthesis.speak(u);
  } catch { /* speech is an enhancement; its absence changes nothing */ }
}

/// The tag to recognise in, from the app's two-letter language.
export const tagFor = l => ({ uk: 'uk-UA', sq: 'sq-AL', en: 'en-GB' }[l] || 'uk-UA');
