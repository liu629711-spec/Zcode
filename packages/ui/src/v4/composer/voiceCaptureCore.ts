// 语音录音的纯函数核心：PCM 浮点降采样 + 16k WAV 编码。
// 不依赖 React 与 @/ 别名，供 node:test 直编直测（见 test/voiceCaptureCore.test.ts）。

/** 任意采样率单声道 Float32 → 16kHz（线性插值，够喂语音识别） */
export function downsampleTo16kMono(samples: Float32Array, sourceRate: number): Float32Array {
  if (sourceRate === 16_000) return samples;
  if (sourceRate <= 0) throw new Error(`非法采样率 ${sourceRate}`);
  const targetLength = Math.max(1, Math.round((samples.length * 16_000) / sourceRate));
  const output = new Float32Array(targetLength);
  const ratio = samples.length / targetLength;
  for (let i = 0; i < targetLength; i += 1) {
    const position = i * ratio;
    const left = Math.floor(position);
    const right = Math.min(left + 1, samples.length - 1);
    const fraction = position - left;
    output[i] = (samples[left] ?? 0) * (1 - fraction) + (samples[right] ?? 0) * fraction;
  }
  return output;
}

/** 44 字节 WAV 头 + 16-bit PCM 小端 */
export function encodeWav16kMono(samples: Float32Array, sourceRate: number): ArrayBuffer {
  const pcm = downsampleTo16kMono(samples, sourceRate);
  const buffer = new ArrayBuffer(44 + pcm.length * 2);
  const view = new DataView(buffer);
  const writeString = (offset: number, text: string) => {
    for (let i = 0; i < text.length; i += 1) view.setUint8(offset + i, text.charCodeAt(i));
  };
  writeString(0, "RIFF");
  view.setUint32(4, 36 + pcm.length * 2, true);
  writeString(8, "WAVE");
  writeString(12, "fmt ");
  view.setUint32(16, 16, true); // fmt chunk 长度
  view.setUint16(20, 1, true); // PCM
  view.setUint16(22, 1, true); // 单声道
  view.setUint32(24, 16_000, true);
  view.setUint32(28, 16_000 * 2, true); // 字节率
  view.setUint16(32, 2, true); // 块对齐
  view.setUint16(34, 16, true); // 位深
  writeString(36, "data");
  view.setUint32(40, pcm.length * 2, true);
  let offset = 44;
  for (const raw of pcm) {
    const clamped = Math.max(-1, Math.min(1, raw));
    view.setInt16(offset, Math.round(clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff), true);
    offset += 2;
  }
  return buffer;
}

/** 一帧 PCM 的 RMS 电平（0..1），驱动录音态的脉冲/波形 UI */
export function computeRmsLevel(samples: Float32Array): number {
  if (samples.length === 0) return 0;
  let sum = 0;
  for (const sample of samples) sum += sample * sample;
  return Math.min(1, Math.sqrt(sum / samples.length) * 4);
}
