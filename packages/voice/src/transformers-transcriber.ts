import type { PcmAudio, Transcriber } from './transcriber.js';
import type { Locale } from './matcher.js';

// Real ASR via Transformers.js (whisper-base). Kept OUT of the static import graph on purpose:
//  - packages/voice must typecheck and build WITHOUT @huggingface/transformers installed (it is a
//    heavy, optional dep). We load it through a NON-LITERAL specifier so tsc types the module as
//    `any` and does not raise TS2307 when the dep is absent — the Phase-0 safety core has zero ML
//    dependency, exactly as the "true-dark = zero bundle delta" invariant (L2) requires.
//  - In the browser the model is fetched + run only after the runtime kill-switch says enabled, so
//    the whole thing lives behind a dynamic import and code-splits out when voice is dark.
//
// Determinism (H1 harness): pinned model revision + greedy decode (no sampling, temperature 0) →
// a fixed WAV + fixed weights transcribe to the same string every run, which is what makes the
// eval-harness report a deterministic gate artifact rather than a flaky number.

/** Whisper's language names, keyed by our storefront locale. whisper-base is multilingual. */
const WHISPER_LANG: Record<Locale, string> = { sq: 'albanian', en: 'english', uk: 'ukrainian' };

export interface TransformersTranscriberOptions {
  /** HF repo id of a Transformers.js-format whisper export. Pinned for reproducibility. */
  readonly model?: string;
  /** Pin a specific model revision (commit/tag) so the weights never drift under the gate. */
  readonly revision?: string;
  /** 'webgpu' in the browser (the only shipped storefront path); 'cpu'/'wasm' for the Node harness. */
  readonly device?: 'webgpu' | 'cpu' | 'wasm' | 'auto';
  /** Weight precision. 'q8' ~ the ~130 MB mixed-precision browser target; 'fp32' for max harness accuracy. */
  readonly dtype?: 'fp32' | 'fp16' | 'q8' | 'q4';
  /** Pre-imported @huggingface/transformers module (browser bundler path). If absent, dynamic-imported. */
  readonly transformersModule?: unknown;
}

const DEFAULTS = {
  model: 'Xenova/whisper-base',
  device: 'auto' as const,
  dtype: 'q8' as const,
};

// Minimal structural types for the slice of the library we touch (avoids a static type dependency).
type AsrPipeline = (audio: PcmAudio, opts: Record<string, unknown>) => Promise<{ text?: string } | { text?: string }[]>;
type TransformersModule = {
  pipeline: (task: string, model: string, opts: Record<string, unknown>) => Promise<AsrPipeline>;
};

export class TransformersTranscriber implements Transcriber {
  readonly #locale: Locale;
  readonly #opts: TransformersTranscriberOptions;
  #pipe: AsrPipeline | null = null;
  #loading: Promise<AsrPipeline> | null = null;

  constructor(locale: Locale, opts: TransformersTranscriberOptions = {}) {
    this.#locale = locale;
    this.#opts = opts;
  }

  /** Load @huggingface/transformers (dynamically, once) and build the pinned ASR pipeline. */
  async #ensure(): Promise<AsrPipeline> {
    if (this.#pipe) return this.#pipe;
    if (this.#loading) return this.#loading;
    this.#loading = (async () => {
      const mod = (this.#opts.transformersModule ?? (await this.#importLib())) as TransformersModule;
      const pipe = await mod.pipeline('automatic-speech-recognition', this.#opts.model ?? DEFAULTS.model, {
        device: this.#opts.device ?? DEFAULTS.device,
        dtype: this.#opts.dtype ?? DEFAULTS.dtype,
        ...(this.#opts.revision ? { revision: this.#opts.revision } : {}),
      });
      this.#pipe = pipe;
      return pipe;
    })();
    return this.#loading;
  }

  // Non-literal specifier: tsc cannot statically resolve it → module typed `any`, no TS2307 when the
  // dep is not installed. Node/the bundler resolves it at runtime when the dep IS present.
  #importLib(): Promise<unknown> {
    const spec = '@huggingface/transformers';
    return import(spec);
  }

  /** Warm the model without a real utterance — the bounded warmup probe (M2). Resolves when ready. */
  async warmup(): Promise<void> {
    await this.#ensure();
  }

  async transcribe(audio: PcmAudio): Promise<string> {
    if (!audio || audio.length === 0) return '';
    const asr = await this.#ensure();
    const out = await asr(audio, {
      language: WHISPER_LANG[this.#locale],
      task: 'transcribe',
      // Greedy, deterministic decode — no sampling, no beam nondeterminism.
      do_sample: false,
      num_beams: 1,
      temperature: 0,
      chunk_length_s: 30,
    });
    const text = Array.isArray(out) ? (out[0]?.text ?? '') : (out.text ?? '');
    return text.trim();
  }
}
