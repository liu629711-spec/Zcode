import { useCallback, useEffect, useRef, useState } from "react";

import { computeRmsLevel, encodeWav16kMono } from "@/v4/composer/voiceCaptureCore.js";

export type VoiceCaptureStatus = "idle" | "requesting" | "recording" | "transcribing";

interface VoiceCaptureOptions {
  /** 录音完成（已编码 16k WAV）后回调；抛错视为转写失败并回 idle */
  onWavReady: (wav: ArrayBuffer) => Promise<void>;
  /** 最长录音时长，超时自动停并转写 */
  maxDurationMs?: number;
}

/**
 * 输入区语音录音状态机（抄 ekko-studio useMicRecorder/usePcmStreamRecorder 的骨架，
 * React 化 + 简化）：点击开始 → getUserMedia 采 PCM → 点击停止 → 降采样 16k 编 WAV →
 * 交 onWavReady（转写）。整段一次转写，无流式/无 VAD。
 *
 * ponytail: ScriptProcessorNode 已废弃但全平台可用；AudioWorklet 需额外模块 URL，不值得。
 */
export function useVoiceCapture({ onWavReady, maxDurationMs = 120_000 }: VoiceCaptureOptions) {
  const [status, setStatus] = useState<VoiceCaptureStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [elapsedMs, setElapsedMs] = useState(0);
  const [level, setLevel] = useState(0);

  const sessionRef = useRef(0);
  const contextRef = useRef<AudioContext | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const chunksRef = useRef<Float32Array[]>([]);
  const startedAtRef = useRef(0);

  const teardown = useCallback(() => {
    streamRef.current?.getTracks().forEach((track) => track.stop());
    streamRef.current = null;
    void contextRef.current?.close().catch(() => undefined);
    contextRef.current = null;
  }, []);

  useEffect(
    () => () => {
      sessionRef.current += 1;
      teardown();
    },
    [teardown],
  );

  const start = useCallback(async () => {
    const session = sessionRef.current + 1;
    sessionRef.current = session;
    setError(null);
    setStatus("requesting");
    setElapsedMs(0);
    setLevel(0);
    chunksRef.current = [];
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          channelCount: 1,
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
      });
      if (sessionRef.current !== session) {
        stream.getTracks().forEach((track) => track.stop());
        return;
      }
      streamRef.current = stream;
      const context = new AudioContext();
      contextRef.current = context;
      const source = context.createMediaStreamSource(stream);
      const processor = context.createScriptProcessor(4096, 1, 1);
      processor.onaudioprocess = (event) => {
        if (sessionRef.current !== session) return;
        const input = event.inputBuffer.getChannelData(0);
        chunksRef.current.push(new Float32Array(input));
        setLevel(computeRmsLevel(input));
      };
      source.connect(processor);
      // ScriptProcessor 需要连到 destination 才会驱动回调；接零增益避免回声
      const mute = context.createGain();
      mute.gain.value = 0;
      processor.connect(mute);
      mute.connect(context.destination);
      startedAtRef.current = Date.now();
      setStatus("recording");
    } catch (cause) {
      teardown();
      setStatus("idle");
      if (sessionRef.current !== session) return;
      setError(
        cause instanceof DOMException && (cause.name === "NotAllowedError" || cause.name === "SecurityError")
          ? "voice.capture.denied"
          : "voice.capture.failed",
      );
    }
  }, [teardown]);

  const stop = useCallback(
    async (mode: "commit" | "cancel") => {
      const session = sessionRef.current;
      if (session === 0 || status !== "recording") return;
      sessionRef.current = session + 1;
      const recordedRate = contextRef.current?.sampleRate ?? 48_000;
      teardown();
      const chunks = chunksRef.current;
      chunksRef.current = [];
      const totalLength = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
      if (mode === "cancel" || totalLength === 0) {
        setStatus("idle");
        setLevel(0);
        return;
      }
      const merged = new Float32Array(totalLength);
      let offset = 0;
      for (const chunk of chunks) {
        merged.set(chunk, offset);
        offset += chunk.length;
      }
      const wav = encodeWav16kMono(merged, recordedRate);
      setStatus("transcribing");
      setLevel(0);
      try {
        await onWavReady(wav);
      } finally {
        setStatus("idle");
        setElapsedMs(0);
      }
    },
    [onWavReady, status, teardown],
  );

  // 录音中计时 + 超长自动停
  useEffect(() => {
    if (status !== "recording") return;
    const timer = setInterval(
      () => setElapsedMs(Date.now() - startedAtRef.current),
      250,
    );
    const autoStop = setTimeout(() => {
      void stop("commit");
    }, maxDurationMs);
    return () => {
      clearInterval(timer);
      clearTimeout(autoStop);
    };
  }, [status, stop, maxDurationMs]);

  return { status, error, clearError: () => setError(null), elapsedMs, level, start, stop };
}
