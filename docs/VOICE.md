# Voice

MVP voice is push-to-talk.

The HUD uses browser/WebView speech APIs when available:

- `SpeechRecognition` or `webkitSpeechRecognition` for transcription
- `speechSynthesis` for spoken responses

The daemon already exposes voice event placeholders. A later native voice core can replace the HUD fallback with:

- microphone capture
- OpenAI Speech-to-Text
- Windows TTS
- wake word detection
- local voice activity detection

Wake word `"NARA"` remains a v0.3 target.
