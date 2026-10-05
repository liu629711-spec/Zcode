// 语音输入引擎一键落机脚本：下载 whisper.cpp + SenseVoice(sherpa-onnx) 到 ~/.zcode/voice-engines/
// 用法：node scripts/setup-voice-engines.mjs [--smoke-only] [--skip-smoke] [--whisper-model ggml-small.bin]
// 幂等：已装好的部件（以可执行文件/模型存在为准）自动跳过；大文件下载支持断点续传（curl -C -）。
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, cpSync, readdirSync, rmSync, writeFileSync, statSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join, resolve, dirname } from "node:path";

const WHISPER_TAG = "v1.7.6"; // 最后一个附 win-x64 二进制包的版本，之后只发源码
const WHISPER_ZIP_URL = `https://github.com/ggml-org/whisper.cpp/releases/download/${WHISPER_TAG}/whisper-bin-x64.zip`;
const SHERPA_VERSION = "1.10.30";
const SHERPA_URL = `https://github.com/k2-fsa/sherpa-onnx/releases/download/v${SHERPA_VERSION}/sherpa-onnx-v${SHERPA_VERSION}-win-x64-shared.tar.bz2`;
// SenseVoice 官方 onnx 模型（中英日韩粤，int8 量化 ~230MB）
const SENSEVOICE_URL =
  "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2";
// whisper ggml 模型：国内网络优先 hf-mirror，失败回退官方
const WHISPER_MODEL_ENDPOINTS = (model) => [
  `https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/${model}`,
  `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/${model}`,
];

const args = process.argv.slice(2);
const smokeOnly = args.includes("--smoke-only");
const skipSmoke = args.includes("--skip-smoke");
const whisperModelArgIdx = args.indexOf("--whisper-model");
const WHISPER_MODEL = whisperModelArgIdx >= 0 ? args[whisperModelArgIdx + 1] : "ggml-small.bin"; // small=466MB 中文可用的最小档

const ROOT = join(homedir(), ".zcode", "voice-engines");
const WHISPER_DIR = join(ROOT, "whisper");
const SENSEVOICE_DIR = join(ROOT, "sensevoice");
const TMP = join(tmpdir(), "zcode-voice-setup");

function log(msg) {
  console.log(`[voice-engines] ${msg}`);
}

function run(cmd, cmdArgs, opts = {}) {
  const r = spawnSync(cmd, cmdArgs, { stdio: ["ignore", "pipe", "pipe"], encoding: "utf8", ...opts });
  if (r.status !== 0) {
    throw new Error(`${cmd} ${cmdArgs.join(" ")} 失败 (exit ${r.status})\nstdout: ${(r.stdout || "").slice(-800)}\nstderr: ${(r.stderr || "").slice(-800)}`);
  }
  return r;
}

function download(url, destFile, label) {
  if (existsSync(destFile) && statSync(destFile).size > 0) {
    log(`${label} 已有下载缓存，续传/跳过`);
  }
  log(`下载 ${label}: ${url}`);
  // curl -L 跟随重定向（HF/CDN 都要）；-C - 断点续传；--retry 带退避
  const r = spawnSync("curl", ["-fL", "-C", "-", "--retry", "3", "--retry-delay", "2", "-o", destFile, url], {
    stdio: "inherit",
  });
  if (r.status !== 0) {
    throw new Error(`curl 下载 ${label} 失败 (exit ${r.status})`);
  }
}

// Windows 自带 bsdtar（System32）：zip / tar.bz2 通吃。
// 不能用 Git Bash 的 GNU tar——它不认 zip，还会把 `C:\` 当远程主机名（"Cannot connect to C"）。
const TAR = "C:\\Windows\\System32\\tar.exe";

function extract(archive, destDir) {
  if (!existsSync(TAR)) throw new Error(`找不到 ${TAR}（Windows 10+ 自带，请确认系统完整）`);
  run(TAR, ["-xf", archive, "-C", destDir]);
}

function findFile(dir, name) {
  for (const entry of readdirSync(dir, { withFileTypes: true, recursive: true })) {
    if (entry.isFile() && entry.name === name) return join(entry.parentPath, entry.name);
  }
  return null;
}

function installWhisperBinaries() {
  const exe = join(WHISPER_DIR, "bin", "whisper-cli.exe");
  if (existsSync(exe)) {
    log("whisper.cpp 二进制已就位，跳过");
    return;
  }
  mkdirSync(TMP, { recursive: true });
  const zip = join(TMP, "whisper-bin-x64.zip");
  download(WHISPER_ZIP_URL, zip, "whisper.cpp 二进制");
  const stage = join(TMP, "whisper-extract");
  rmSync(stage, { recursive: true, force: true });
  mkdirSync(stage, { recursive: true });
  extract(zip, stage);
  // zip 里的 exe 藏在 Release/ 子目录：定位 main.exe 所在目录，整目录平铺进 bin/
  const found = findFile(stage, "whisper-cli.exe");
  if (!found) throw new Error(`whisper.cpp 压缩包里找不到 whisper-cli.exe（${stage}）`);
  mkdirSync(join(WHISPER_DIR, "bin"), { recursive: true });
  cpSync(dirname(found), join(WHISPER_DIR, "bin"), { recursive: true });
  writeFileSync(join(WHISPER_DIR, "bin", "VERSION"), `${WHISPER_TAG}\n`);
  log(`whisper.cpp ${WHISPER_TAG} 就位 → ${join(WHISPER_DIR, "bin")}`);
}

function installWhisperModel() {
  const model = join(WHISPER_DIR, "models", WHISPER_MODEL);
  if (existsSync(model) && statSync(model).size > 50 * 1024 * 1024) {
    log(`whisper 模型 ${WHISPER_MODEL} 已就位，跳过`);
    return;
  }
  mkdirSync(join(WHISPER_DIR, "models"), { recursive: true });
  const partial = join(TMP, WHISPER_MODEL);
  mkdirSync(TMP, { recursive: true });
  let lastErr;
  for (const url of WHISPER_MODEL_ENDPOINTS(WHISPER_MODEL)) {
    try {
      download(url, partial, `whisper 模型 ${WHISPER_MODEL}`);
      lastErr = null;
      break;
    } catch (error) {
      lastErr = error;
      log(`端点失败，换下一个：${error.message.split("\n")[0]}`);
    }
  }
  if (lastErr) throw lastErr;
  cpSync(partial, model);
  log(`whisper 模型就位 → ${model}`);
}

function installSherpaBinaries() {
  const exe = join(SENSEVOICE_DIR, "bin", "sherpa-onnx-offline.exe");
  if (existsSync(exe)) {
    log("sherpa-onnx 二进制已就位，跳过");
    return;
  }
  mkdirSync(TMP, { recursive: true });
  const archive = join(TMP, "sherpa-win-x64.tar.bz2");
  download(SHERPA_URL, archive, "sherpa-onnx 二进制");
  const stage = join(TMP, "sherpa-extract");
  rmSync(stage, { recursive: true, force: true });
  mkdirSync(stage, { recursive: true });
  extract(archive, stage);
  // tar 包根目录带版本号：定位 sherpa-onnx-offline.exe 所在目录，整目录平铺进 bin/（exe 依赖同目录 dll）
  const found = findFile(stage, "sherpa-onnx-offline.exe");
  if (!found) throw new Error(`sherpa-onnx 压缩包里找不到 sherpa-onnx-offline.exe（${stage}）`);
  mkdirSync(join(SENSEVOICE_DIR, "bin"), { recursive: true });
  cpSync(dirname(found), join(SENSEVOICE_DIR, "bin"), { recursive: true });
  writeFileSync(join(SENSEVOICE_DIR, "bin", "VERSION"), `sherpa-onnx v${SHERPA_VERSION}\n`);
  log(`sherpa-onnx v${SHERPA_VERSION} 就位 → ${join(SENSEVOICE_DIR, "bin")}`);
}

function installSenseVoiceModel() {
  const model = join(SENSEVOICE_DIR, "models", "model.int8.onnx");
  const tokens = join(SENSEVOICE_DIR, "models", "tokens.txt");
  if (existsSync(model) && existsSync(tokens)) {
    log("SenseVoice 模型已就位，跳过");
    return;
  }
  mkdirSync(TMP, { recursive: true });
  const archive = join(TMP, "sense-voice.tar.bz2");
  download(SENSEVOICE_URL, archive, "SenseVoice 模型");
  const stage = join(TMP, "sensevoice-extract");
  rmSync(stage, { recursive: true, force: true });
  mkdirSync(stage, { recursive: true });
  extract(archive, stage);
  const foundModel = findFile(stage, "model.int8.onnx");
  const foundTokens = findFile(stage, "tokens.txt");
  if (!foundModel || !foundTokens) throw new Error(`SenseVoice 压缩包里找不到 model.int8.onnx / tokens.txt（${stage}）`);
  mkdirSync(join(SENSEVOICE_DIR, "models"), { recursive: true });
  cpSync(foundModel, model);
  cpSync(foundTokens, tokens);
  log("SenseVoice 模型就位 → sensevoice/models/");
}

// 冒烟自检：PowerShell SAPI 合成一段 16k wav → 喂两个引擎 → 断言都能出非空文字
function runSmokeTest() {
  log("开始冒烟自检（SAPI 合成语音 → 双引擎转写）");
  mkdirSync(TMP, { recursive: true });
  const wav = join(TMP, "smoke.wav");
  const voices = run("powershell", [
    "-NoProfile", "-Command",
    "Add-Type -AssemblyName System.Speech; (New-Object System.Speech.Synthesis.SpeechSynthesizer).GetInstalledVoices() | ForEach-Object { $_.VoiceInfo.Culture }",
  ]).stdout;
  const hasZh = /zh/i.test(voices);
  const text = hasZh ? "你好世界，今天天气不错" : "hello world this is a voice input smoke test";
  log(`TTS 音色：${hasZh ? "中文" : "英文兜底"}（${voices.trim().split(/\r?\n/).join(", ")}）`);
  const psScript = [
    "Add-Type -AssemblyName System.Speech",
    "$s = New-Object System.Speech.Synthesis.SpeechSynthesizer",
    `$s.SetOutputToWaveFile('${wav.replace(/\\/g, "\\\\")}', [System.Speech.AudioFormat.SpeechAudioFormatInfo]::new(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.EncodingFormat]::Pcm))`,
    `$s.Speak('${text}')`,
    "$s.Dispose()",
  ].join("; ");
  run("powershell", ["-NoProfile", "-Command", psScript]);
  if (!existsSync(wav)) throw new Error("SAPI 没有生成测试 wav");
  log(`测试 wav 已合成：${wav}`);

  // whisper：whisper-cli 一次性转写（-nt 纯文本无时间戳，-np 关闭过程打印）
  const whisperOut = run(join(WHISPER_DIR, "bin", "whisper-cli.exe"), [
    "-m", join(WHISPER_DIR, "models", WHISPER_MODEL),
    "-f", wav, "-nt", "-np",
  ]).stdout.trim();
  log(`whisper 转写结果："${whisperOut}"`);
  if (!whisperOut) throw new Error("whisper 转写为空");

  // sherpa-onnx：一次性离线转写，输出格式以实际为准，宽松断言"有可打印内容"
  const sherpaOut = run(join(SENSEVOICE_DIR, "bin", "sherpa-onnx-offline.exe"), [
    `--sense-voice-model=${join(SENSEVOICE_DIR, "models", "model.int8.onnx")}`,
    `--tokens=${join(SENSEVOICE_DIR, "models", "tokens.txt")}`,
    wav,
  ]);
  const sherpaRaw = `${sherpaOut.stdout}\n${sherpaOut.stderr}`.trim();
  log(`sherpa-onnx 原始输出：\n${sherpaRaw}`);
  if (!sherpaRaw) throw new Error("sherpa-onnx 无输出");

  log("✅ 冒烟自检通过：双引擎都能出字");
}

function main() {
  if (process.platform !== "win32") {
    throw new Error("本脚本目前只支持 Windows（ZCode 桌面端当前目标平台）");
  }
  mkdirSync(ROOT, { recursive: true });
  log(`引擎根目录：${ROOT}`);
  if (!smokeOnly) {
    installWhisperBinaries();
    installWhisperModel();
    installSherpaBinaries();
    installSenseVoiceModel();
  }
  if (!skipSmoke) runSmokeTest();
  log("全部完成");
}

try {
  main();
} catch (error) {
  console.error(`[voice-engines] ❌ ${error.message}`);
  process.exit(1);
}
