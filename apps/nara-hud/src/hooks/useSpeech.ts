import { useCallback, useMemo, useRef, useState } from "react";

interface SpeechRecognitionResult {
  readonly transcript: string;
}

interface SpeechRecognitionAlternativeList {
  readonly [index: number]: SpeechRecognitionResult;
}

interface SpeechRecognitionResultList {
  readonly [index: number]: SpeechRecognitionAlternativeList;
  readonly length: number;
}

interface SpeechRecognitionEvent extends Event {
  readonly results: SpeechRecognitionResultList;
}

interface SpeechRecognitionLike extends EventTarget {
  lang: string;
  interimResults: boolean;
  continuous: boolean;
  start: () => void;
  stop: () => void;
  onresult: ((event: SpeechRecognitionEvent) => void) | null;
  onerror: (() => void) | null;
  onend: (() => void) | null;
}

type SpeechRecognitionConstructor = new () => SpeechRecognitionLike;

declare global {
  interface Window {
    SpeechRecognition?: SpeechRecognitionConstructor;
    webkitSpeechRecognition?: SpeechRecognitionConstructor;
  }
}

export function useSpeech() {
  const recognitionRef = useRef<SpeechRecognitionLike | null>(null);
  const [listening, setListening] = useState(false);
  const [transcript, setTranscript] = useState("");

  const recognitionConstructor = useMemo(
    () => window.SpeechRecognition ?? window.webkitSpeechRecognition,
    []
  );

  const supported = Boolean(recognitionConstructor);
  const speechSupported = typeof window.speechSynthesis !== "undefined";

  const stop = useCallback(() => {
    recognitionRef.current?.stop();
    recognitionRef.current = null;
    setListening(false);
  }, []);

  const start = useCallback(
    (onFinal: (text: string) => void) => {
      if (!recognitionConstructor || listening) {
        return;
      }

      const recognition = new recognitionConstructor();
      recognitionRef.current = recognition;
      recognition.lang = "id-ID";
      recognition.continuous = false;
      recognition.interimResults = false;

      recognition.onresult = (event) => {
        const text = Array.from({ length: event.results.length })
          .map((_, index) => event.results[index][0]?.transcript ?? "")
          .join(" ")
          .trim();
        setTranscript(text);
        if (text) {
          onFinal(text);
        }
      };

      recognition.onerror = () => {
        setListening(false);
      };

      recognition.onend = () => {
        setListening(false);
        recognitionRef.current = null;
      };

      setTranscript("");
      setListening(true);
      recognition.start();
    },
    [listening, recognitionConstructor]
  );

  const speak = useCallback((text: string) => {
    if (!window.speechSynthesis || !text.trim()) {
      return;
    }

    window.speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance(text.slice(0, 800));
    utterance.lang = "id-ID";
    utterance.rate = 0.96;
    window.speechSynthesis.speak(utterance);
  }, []);

  const stopSpeaking = useCallback(() => {
    window.speechSynthesis?.cancel();
  }, []);

  return {
    listening,
    transcript,
    supported,
    speechSupported,
    start,
    stop,
    speak,
    stopSpeaking
  };
}
