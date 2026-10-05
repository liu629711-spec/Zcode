// ============================================================
// 语音录音核心的可运行检查（node:test + node:assert/strict，无假件）：
//  1. WAV 编码：44 字节头逐字段正确（RIFF/WAVE/fmt PCM/16k/单声道/16bit），
//     data 块长度 = 样本数 × 2，总长 = 44 + data；
//  2. 降采样：48k → 16k 长度按比例收拢；恒等率直通；振幅不越界（±1 截断）；
//  3. RMS 电平：静音 → 0；半幅正弦 → >0 且 ≤1。
//
// 运行（照 councilMeeting.test.ts 的 tsc 直编 + node:test）：
//   T=$(mktemp -d) && node node_modules/typescript/bin/tsc \
//     packages/ui/test/voiceCaptureCore.test.ts \
//     --outDir "$T" --rootDir . --module nodenext --moduleResolution nodenext \
//     --target es2023 --skipLibCheck --strict --types node && \
//   node --test "$T/packages/ui/test/voiceCaptureCore.test.js"
// ============================================================
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  computeRmsLevel,
  downsampleTo16kMono,
  encodeWav16kMono,
} from "../src/v4/composer/voiceCaptureCore.js";

function readString(view: DataView, offset: number, length: number): string {
  let text = "";
  for (let i = 0; i < length; i += 1) text += String.fromCharCode(view.getUint8(offset + i));
  return text;
}

test("encodeWav16kMono 写出合法 44 字节 WAV 头 + PCM data 块", () => {
  const samples = new Float32Array(16_000); // 1 秒 @48k 输入
  const wav = encodeWav16kMono(samples, 48_000);
  const view = new DataView(wav);

  assert.equal(readString(view, 0, 4), "RIFF");
  assert.equal(readString(view, 8, 4), "WAVE");
  assert.equal(readString(view, 12, 4), "fmt ");
  assert.equal(view.getUint32(16, true), 16); // fmt chunk 长度
  assert.equal(view.getUint16(20, true), 1); // PCM
  assert.equal(view.getUint16(22, true), 1); // 单声道
  assert.equal(view.getUint32(24, true), 16_000); // 采样率
  assert.equal(view.getUint16(34, true), 16); // 位深
  assert.equal(readString(view, 36, 4), "data");

  const dataLength = view.getUint32(40, true);
  // 48k 1 秒降采样到 16k ≈ 16_000 样本（线性插值按 round 收拢）
  assert.equal(dataLength % 2, 0);
  assert.equal(wav.byteLength, 44 + dataLength);
  assert.equal(dataLength / 2, Math.round((16_000 * 16_000) / 48_000));
});

test("降采样：长度按比例收拢，同率直通（不截断，截断归编码）", () => {
  const sine = new Float32Array(48_000);
  for (let i = 0; i < sine.length; i += 1) sine[i] = Math.sin((i / 48_000) * 2 * Math.PI * 440);
  const down = downsampleTo16kMono(sine, 48_000);
  assert.equal(down.length, 16_000);

  const identity = downsampleTo16kMono(sine, 16_000);
  assert.ok(identity === sine);
});

test("编码：越界样本截断到 ±1 后写入 PCM", () => {
  const wav = new DataView(encodeWav16kMono(new Float32Array([2, -2, 0.5]), 16_000));
  assert.equal(wav.getInt16(44, true), 0x7fff);
  assert.equal(wav.getInt16(46, true), -0x8000);
  assert.equal(wav.getInt16(48, true), Math.round(0.5 * 0x7fff));
});

test("RMS 电平：静音为 0，有声为正且不超过 1", () => {
  assert.equal(computeRmsLevel(new Float32Array(4_096)), 0);
  const sine = new Float32Array(4_096);
  for (let i = 0; i < sine.length; i += 1) sine[i] = 0.5 * Math.sin(i / 10);
  const level = computeRmsLevel(sine);
  assert.ok(level > 0 && level <= 1, `level=${level}`);
});
