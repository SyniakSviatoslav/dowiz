import { useState, useCallback, useRef } from 'react';

export type VoiceStatus = 'idle' | 'listening' | 'processing' | 'error' | 'unsupported';

export interface VoiceResult {
  transcript: string;
  confidence: number;
}

interface SpeechRecognitionEvent extends Event {
  results: SpeechRecognitionResultList;
}

interface SpeechRecognitionErrorEvent extends Event {
  error: string;
}

interface SpeechRecognition extends EventTarget {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  start(): void;
  stop(): void;
  abort(): void;
  onresult: ((event: SpeechRecognitionEvent) => void) | null;
  onerror: ((event: SpeechRecognitionErrorEvent) => void) | null;
  onend: (() => void) | null;
}

const SpeechRecognitionAPI =
  typeof window !== 'undefined'
    ? (window as unknown as Record<string, new () => SpeechRecognition>).SpeechRecognition ??
      (window as unknown as Record<string, new () => SpeechRecognition>).webkitSpeechRecognition
    : undefined;

export function useVoiceOrder(lang = 'sq-AL') {
  const [status, setStatus] = useState<VoiceStatus>('idle');
  const [transcript, setTranscript] = useState('');
  const recognitionRef = useRef<SpeechRecognition | null>(null);
  const resolveRef = useRef<((result: VoiceResult) => void) | null>(null);
  const rejectRef = useRef<((err: Error) => void) | null>(null);

  const startListening = useCallback(() => {
    if (!SpeechRecognitionAPI) {
      setStatus('unsupported');
      return Promise.reject(new Error('Speech recognition not supported'));
    }

    return new Promise<VoiceResult>((resolve, reject) => {
      try {
        const recognition = new SpeechRecognitionAPI();
        recognition.continuous = false;
        recognition.interimResults = true;
        recognition.lang = lang;

        recognition.onresult = (event: SpeechRecognitionEvent) => {
          const last = event.results.length - 1;
          const result = event.results[last];
          if (!result) return;
          const transcriptText = result[0]?.transcript ?? '';
          setTranscript(transcriptText);
          setStatus(result.isFinal ? 'processing' : 'listening');

          if (result.isFinal) {
            resolve({ transcript: transcriptText, confidence: result[0]?.confidence ?? 0 });
          }
        };

        recognition.onerror = (event: SpeechRecognitionErrorEvent) => {
          setStatus('error');
          reject(new Error(event.error));
        };

        recognition.onend = () => {
          setStatus('idle');
        };

        recognition.start();
        recognitionRef.current = recognition;
        setStatus('listening');
        resolveRef.current = resolve;
        rejectRef.current = reject;
      } catch (err) {
        setStatus('error');
        reject(err instanceof Error ? err : new Error('Failed to start speech recognition'));
      }
    });
  }, [lang]);

  const stopListening = useCallback(() => {
    recognitionRef.current?.stop();
    recognitionRef.current = null;
    setStatus('idle');
  }, []);

  return { status, transcript, startListening, stopListening, isSupported: !!SpeechRecognitionAPI } as const;
}
