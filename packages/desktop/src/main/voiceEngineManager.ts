import type {
  VoiceEngineStatus,
  VoiceTranscribeRequest,
  VoiceTranscribeResult,
} from "@zcode/shared";
import { PlatformChannels } from "@zcode/shared";
import { ipcMain, session, systemPreferences } from "electron";
import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { spawn } from "node:child_process";

/**
 * 语音输入引擎管理：渲染进程录好 16k WAV，这里按供应商路由转写。
 *
 * - sensevoice：每次起 `sherpa-onnx-offline.exe` 子进程（模型 int8 冷启 ~1s，实测转写 RTF≈0.013）
 * - whisper：每次起 `whisper-cli.exe` 子进程（ggml-small 冷启数秒）
 * - cloud：OpenAI 兼容 /v1/audio/transcriptions，主进程直传（key 不进渲染进程闭包之外）
 *
 * ponytail: 双引擎都是逐次冷启（无常驻服务/端口管理）。实测短语音体感可接受；
 * 若真机反馈启动慢，升级路径 = whisper-server.exe / sherpa 常驻进程 + 空闲退出。
 */

const ENGINE_ROOT = join(homedir(), ".zcode", "voice-engines");
const WHISPER_BIN = join(ENGINE_ROOT, "whisper", "bin");
const WHISPER_MODELS = join(ENGINE_ROOT, "whisper", "models");
const SENSEVOICE_BIN = join(ENGINE_ROOT, "sensevoice", "bin");
const SENSEVOICE_MODELS = join(ENGINE_ROOT, "sensevoice", "models");

/** 转写全局串行化：本地引擎吃满核会拖慢整机，排队即可 */
let transcribeQueue: Promise<unknown> = Promise.resolve();

interface VoiceLogger {
  warn: (...args: unknown[]) => void;
  info: (...args: unknown[]) => void;
}

function firstFile(dir: string, predicate: (name: string) => boolean): string | undefined {
  if (!existsSync(dir)) return undefined;
  const hit = readdirSync(dir).find(predicate);
  return hit ? join(dir, hit) : undefined;
}

function resolveWhisperModel(): string | undefined {
  return firstFile(WHISPER_MODELS, (name) => name.endsWith(".bin"));
}

export function getVoiceEngineStatus(): VoiceEngineStatus {
  const whisperModel = resolveWhisperModel();
  const senseVoiceModel = firstFile(SENSEVOICE_MODELS, (name) => name === "model.int8.onnx");
  return {
    whisper: {
      installed: existsSync(join(WHISPER_BIN, "whisper-cli.exe")) && Boolean(whisperModel),
      modelPath: whisperModel,
    },
    sensevoice: {
      installed:
        existsSync(join(SENSEVOICE_BIN, "sherpa-onnx-offline.exe")) && Boolean(senseVoiceModel),
      modelPath: senseVoiceModel,
    },
  };
}

/** BCP-47 → 引擎语言码（"zh-CN" → "zh"；缺省交给引擎 auto） */
function normalizeEngineLanguage(language?: string): string | undefined {
  const primary = language?.trim().split("-")[0]?.toLowerCase();
  return primary && primary.length === 2 ? primary : undefined;
}

function runEngineProcess(
  exePath: string,
  args: string[],
  options: { timeoutMs: number; logger: VoiceLogger },
): Promise<{ code: number | null; stdout: string; stderr: string }> {
  return new Promise((resolvePromise) => {
    const child = spawn(exePath, args, { windowsHide: true });
    let stdout = "";
    let stderr = "";
    let settled = false;
    const timer = setTimeout(() => {
      if (!settled) child.kill();
    }, options.timeoutMs);
    child.stdout.on("data", (chunk: Buffer) => {
      stdout += chunk.toString("utf8");
    });
    child.stderr.on("data", (chunk: Buffer) => {
      stderr += chunk.toString("utf8");
    });
    child.on("error", (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolvePromise({ code: null, stdout, stderr: `${stderr}\n${String(error)}` });
    });
    child.on("close", (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolvePromise({ code, stdout, stderr });
    });
  });
}

/** sherpa-onnx 末行输出 JSON（{"lang":..., "text":...}），取最后一个可解析行 */
function parseSenseVoiceOutput(raw: string): string | null {
  const lines = raw.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line || !line.startsWith("{")) continue;
    try {
      const parsed = JSON.parse(line) as { text?: string };
      if (typeof parsed.text === "string") return parsed.text.trim();
    } catch {
      // 当前行不一定是完整 JSON（进度混写），继续向前找
    }
  }
  return null;
}

async function transcribeWithSenseVoice(
  wavPath: string,
  logger: VoiceLogger,
): Promise<string> {
  const modelPath = firstFile(SENSEVOICE_MODELS, (name) => name === "model.int8.onnx");
  const tokensPath = firstFile(SENSEVOICE_MODELS, (name) => name === "tokens.txt");
  if (!modelPath || !tokensPath) throw new Error("SenseVoice 模型未安装（~/.zcode/voice-engines/sensevoice/models/）");
  const result = await runEngineProcess(
    join(SENSEVOICE_BIN, "sherpa-onnx-offline.exe"),
    [
      `--sense-voice-model=${modelPath}`,
      `--tokens=${tokensPath}`,
      wavPath,
    ],
    { timeoutMs: 120_000, logger },
  );
  if (result.code !== 0) {
    throw new Error(`sherpa-onnx-offline 退出码 ${result.code}: ${result.stderr.slice(-300)}`);
  }
  const text = parseSenseVoiceOutput(result.stdout);
  if (text === null) throw new Error("SenseVoice 输出解析失败");
  return text;
}

async function transcribeWithWhisper(
  wavPath: string,
  language: string | undefined,
  logger: VoiceLogger,
): Promise<string> {
  const modelPath = resolveWhisperModel();
  if (!modelPath) throw new Error("whisper 模型未安装（~/.zcode/voice-engines/whisper/models/）");
  const args = ["-m", modelPath, "-f", wavPath, "-nt", "-np"];
  if (language) args.push("--language", language);
  const result = await runEngineProcess(join(WHISPER_BIN, "whisper-cli.exe"), args, {
    timeoutMs: 180_000,
    logger,
  });
  if (result.code !== 0) {
    throw new Error(`whisper-cli 退出码 ${result.code}: ${result.stderr.slice(-300)}`);
  }
  const text = result.stdout.trim();
  if (!text) throw new Error("whisper 转写为空");
  return text;
}

/** OpenAI 兼容端点：容忍 baseUrl 带/不带 /v1 后缀 */
function resolveCloudEndpoint(baseUrl: string): string {
  const trimmed = baseUrl.trim().replace(/\/+$/, "");
  if (trimmed.endsWith("/v1/audio/transcriptions")) return trimmed;
  if (trimmed.endsWith("/v1")) return `${trimmed}/audio/transcriptions`;
  return `${trimmed}/v1/audio/transcriptions`;
}

async function transcribeWithCloud(
  audio: ArrayBuffer,
  cloud: NonNullable<VoiceTranscribeRequest["cloud"]>,
  language: string | undefined,
): Promise<string> {
  if (!cloud.baseUrl?.trim() || !cloud.model?.trim()) {
    throw new Error("云转写配置不完整：需要 baseUrl 和 model");
  }
  const form = new FormData();
  form.append("file", new Blob([audio], { type: "audio/wav" }), "speech.wav");
  form.append("model", cloud.model.trim());
  if (language) form.append("language", language);
  const response = await fetch(resolveCloudEndpoint(cloud.baseUrl), {
    method: "POST",
    headers: { Authorization: `Bearer ${cloud.apiKey ?? ""}` },
    body: form,
    signal: AbortSignal.timeout(60_000),
  });
  if (!response.ok) {
    const detail = (await response.text()).slice(0, 300);
    throw new Error(`云转写失败 HTTP ${response.status}: ${detail}`);
  }
  const payload = (await response.json()) as { text?: string };
  const text = payload.text?.trim() ?? "";
  if (!text) throw new Error("云转写返回空文本");
  return text;
}

export function registerVoiceIpcHandlers(logger: VoiceLogger): void {
  ipcMain.handle(
    PlatformChannels.VoiceTranscribe,
    async (_event, request: VoiceTranscribeRequest): Promise<VoiceTranscribeResult> => {
      const run = transcribeQueue.then(async () => {
        const startedAt = Date.now();
        const language = normalizeEngineLanguage(request.language);
        try {
          let text: string;
          if (request.provider === "cloud") {
            // 云 API 走 HTTP multipart，无需临时文件
            text = await transcribeWithCloud(request.audio, request.cloud!, language);
          } else {
            // 本地引擎子进程按路径读 WAV：临时落盘，转写完立刻删
            const tempDir = mkdtempSync(join(tmpdir(), "zcode-voice-"));
            const wavPath = join(tempDir, "speech.wav");
            try {
              writeFileSync(wavPath, Buffer.from(request.audio));
              text =
                request.provider === "sensevoice"
                  ? await transcribeWithSenseVoice(wavPath, logger)
                  : await transcribeWithWhisper(wavPath, language, logger);
            } finally {
              rmSync(tempDir, { recursive: true, force: true });
            }
          }
          return { text, provider: request.provider, durationMs: Date.now() - startedAt };
        } catch (error) {
          logger.warn("[voice] 转写失败", {
            provider: request.provider,
            error: error instanceof Error ? error.message : String(error),
          });
          throw error;
        }
      }) as Promise<VoiceTranscribeResult>;
      // 队列自愈：当前任务失败不打断后续转写
      transcribeQueue = run.catch(() => undefined);
      return run;
    },
  );

  ipcMain.handle(PlatformChannels.VoiceGetEngineStatus, () => getVoiceEngineStatus());

  ipcMain.handle(PlatformChannels.VoiceRequestMicrophoneAccess, async () => {
    if (process.platform !== "darwin") return true;
    if (typeof systemPreferences.askForMediaAccess !== "function") return true;
    return systemPreferences.askForMediaAccess("microphone");
  });
}

/**
 * Chromium 媒体采集权限放行。必须同时装 request + check 两个 handler：
 * Windows 上 Chromium 先走同步 check，缺省 false 会直接拒掉 getUserMedia，
 * request handler 根本轮不到执行（hermes-agent 实证）。
 */
export function installVoiceMediaPermissions(): void {
  session.defaultSession.setPermissionRequestHandler((_webContents, permission, callback) => {
    callback(permission === "media");
  });
  session.defaultSession.setPermissionCheckHandler(
    (_webContents, permission) => permission === "media",
  );
}
