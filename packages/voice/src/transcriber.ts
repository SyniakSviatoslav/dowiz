// The ASR port — the ONLY audio→text surface the engine depends on. Deliberately an OBJECT with a
// method, NOT a bare `(audio) => Promise<string>` parameter: the engine's public API accepts no
// function-typed parameter (M3 / R2-F), so nothing write-capable can be closed over and smuggled in.
// A Transcriber reads audio and returns text; it holds no menu setter and no store dispatch. The two
// implementations — TransformersTranscriber (real whisper) and a test FakeTranscriber — are
// interchangeable, which is exactly what lets the WhisperProvider loop be unit-tested without a model.
//
// Audio contract: mono 16 kHz PCM as Float32 in [-1, 1] — the format Transformers.js whisper expects
// and the format the browser's Web Audio graph (AudioContext @ 16 kHz) and a decoded WAV both produce.

/** One utterance of decoded PCM audio: mono, 16 kHz, Float32 samples in [-1, 1]. */
export type PcmAudio = Float32Array;

export interface Transcriber {
  /**
   * Transcribe one utterance to text. Best-effort: returns '' when nothing intelligible is heard
   * (silence / noise) — never throws for empty audio. Deterministic implementations pin the decode
   * (greedy, temperature 0) so a fixed WAV + pinned weights yield the same string every run.
   */
  transcribe(audio: PcmAudio): Promise<string>;
}
