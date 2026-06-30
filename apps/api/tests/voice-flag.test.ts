import { describe, it, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { isVoiceEnabled } from '../src/lib/voice-flag.js';

// Guardrail (council voice-control / ADR-0015 §9) — the voice kill-switch is FAIL-CLOSED.
// RED if voice could read as enabled from an unset/garbage flag, or if VOICE_KILL fails to override
// the launch flag. This predicate is the single source of truth for both /api/public/voice-config
// and the CSP connect-src R2 widening (breaker R2-E), so its correctness gates the whole runtime kill.

const SAVED_ENABLED = process.env.VOICE_CONTROL_ENABLED;
const SAVED_KILL = process.env.VOICE_KILL;

function set(name: string, value: string | undefined): void {
  if (value === undefined) delete process.env[name];
  else process.env[name] = value;
}

describe('isVoiceEnabled — fail-closed voice kill-switch', () => {
  afterEach(() => {
    set('VOICE_CONTROL_ENABLED', SAVED_ENABLED);
    set('VOICE_KILL', SAVED_KILL);
  });

  it('OFF when the launch flag is unset (default dark)', () => {
    set('VOICE_CONTROL_ENABLED', undefined);
    set('VOICE_KILL', undefined);
    assert.equal(isVoiceEnabled(), false);
  });

  it('OFF for any value other than the literal "true" (no truthy coercion)', () => {
    set('VOICE_KILL', undefined);
    for (const v of ['1', 'TRUE', 'yes', 'on', '']) {
      set('VOICE_CONTROL_ENABLED', v);
      assert.equal(isVoiceEnabled(), false, `"${v}" must not enable voice`);
    }
  });

  it('ON only when the launch flag === "true" and the kill is not set', () => {
    set('VOICE_CONTROL_ENABLED', 'true');
    set('VOICE_KILL', undefined);
    assert.equal(isVoiceEnabled(), true);
  });

  it('the runtime kill OVERRIDES an enabled launch flag', () => {
    set('VOICE_CONTROL_ENABLED', 'true');
    set('VOICE_KILL', 'true');
    assert.equal(isVoiceEnabled(), false, 'VOICE_KILL must force voice off across all clients');
  });
});
