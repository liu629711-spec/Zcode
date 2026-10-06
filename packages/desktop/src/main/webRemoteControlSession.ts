import type { WebRemoteControlSessionState } from "@zcode/shared";
import { HostMessageTypes, HostResponseTypes, PlatformChannels } from "@zcode/shared";
import { app, ipcMain, type UtilityProcess } from "electron";
import { randomBytes } from "node:crypto";
import { mkdirSync, writeFileSync } from "node:fs";
import { networkInterfaces } from "node:os";
import { join } from "node:path";
import { createServer, type AddressInfo } from "node:net";

type ElectronUtilityProcess = UtilityProcess;

/**
 * Web 远程控制会话（main 侧编排）：弹窗点"开始"→ 这里挑空闲端口、探局域网 IPv4、
 * 生成配对 token → 转告窗口 Host 挂 createHttpServer（手机连到的就是同一个工作区
 * 引擎）。停止 = Host 关监听 + 会话作废。renderer 按 webContentsId 轮询会话状态。
 *
 * ponytail: 不维护手机连接计数（"已就绪"本就是服务就绪态，批 3 再补）；Host 绑定
 * 失败经 requestId 一次性回执兜底；跨网隧道 v1 不做，文档引导 cloudflared/Tailscale。
 */

interface WebRemoteControlLogger {
  info: (...args: unknown[]) => void;
  warn: (...args: unknown[]) => void;
}

interface WebRemoteControlSessionOptions {
  windowHostProcessMap: Map<number, ElectronUtilityProcess>;
  logger: WebRemoteControlLogger;
}

const IDLE_STATE: WebRemoteControlSessionState = { active: false };

/** webContentsId → 当前会话 */
const sessionsByWebContentsId = new Map<
  number,
  { state: WebRemoteControlSessionState; hostChild: ElectronUtilityProcess }
>();

/** 虚拟网卡名黑名单：探局域网 IP 时跳过，避免二维码给出连不上的地址 */
const VIRTUAL_ADAPTER_PATTERN = /virtual|vmware|vbox|hyper-v|wsl|loopback|tap|tun|vethernet|bluetooth/i;

function pickLanIpv4(): string | undefined {
  const candidates: string[] = [];
  for (const [name, addresses] of Object.entries(networkInterfaces())) {
    if (VIRTUAL_ADAPTER_PATTERN.test(name)) continue;
    for (const address of addresses ?? []) {
      if (address.family !== "IPv4" || address.internal) continue;
      // 私网段优先，其余（罕见公网地址）排后
      if (/^(192\.168\.|10\.|172\.(1[6-9]|2\d|3[01])\.)/.test(address.address)) {
        candidates.unshift(address.address);
      } else {
        candidates.push(address.address);
      }
    }
  }
  return candidates[0];
}

function pickFreePort(): Promise<number> {
  return new Promise((resolvePromise, rejectPromise) => {
    const probe = createServer();
    probe.unref();
    probe.on("error", rejectPromise);
    probe.listen(0, "0.0.0.0", () => {
      const port = (probe.address() as AddressInfo).port;
      probe.close(() => resolvePromise(port));
    });
  });
}

/** 批 1 占位页；批 2 的手机控制台 bundle 会写到同一目录顶掉它 */
function ensurePlaceholderStaticRoot(logger: WebRemoteControlLogger): string {
  const root = join(app.getPath("userData"), "web-remote-control", "static");
  try {
    mkdirSync(root, { recursive: true });
    writeFileSync(
      join(root, "index.html"),
      `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8">` +
        `<meta name="viewport" content="width=device-width, initial-scale=1">` +
        `<title>ZCode 工作区</title></head>` +
        `<body style="font-family:system-ui;display:flex;min-height:100vh;align-items:center;justify-content:center;margin:0;background:#1b1917;color:#e8e4de">` +
        `<div style="text-align:center;padding:2rem"><div style="font-size:2rem">&#9989;</div>` +
        `<h1 style="font-size:1.1rem;font-weight:600">已连接到工作区服务</h1>` +
        `<p style="opacity:.65;font-size:.9rem">手机控制台正在路上</p></div></body></html>`,
      "utf8",
    );
  } catch (error) {
    logger.warn("[web-remote] 占位页写入失败", {
      error: error instanceof Error ? error.message : String(error),
    });
  }
  return root;
}

function waitForHostAck(
  hostChild: ElectronUtilityProcess,
  requestId: string,
  timeoutMs: number,
): Promise<{ ok: boolean; error?: string }> {
  return new Promise((resolvePromise) => {
    const timer = setTimeout(() => {
      hostChild.removeListener("message", onMessage);
      resolvePromise({ ok: false, error: "窗口 Host 未响应启动请求" });
    }, timeoutMs);
    const onMessage = (payload: unknown) => {
      const message = payload as { type?: string; requestId?: string; ok?: boolean; error?: string };
      if (message?.type !== HostResponseTypes.WebRemoteControlState) return;
      if (message.requestId !== requestId) return;
      clearTimeout(timer);
      hostChild.removeListener("message", onMessage);
      resolvePromise({ ok: message.ok === true, error: message.error });
    };
    hostChild.on("message", onMessage);
  });
}

async function startSession(
  senderId: number,
  options: WebRemoteControlSessionOptions,
): Promise<WebRemoteControlSessionState> {
  const existing = sessionsByWebContentsId.get(senderId);
  if (existing?.state.active) return existing.state;

  // 弹窗跑在工作区窗口 renderer 里，senderId 就是 windowHostProcessMap 的键
  const hostChild = options.windowHostProcessMap.get(senderId);
  if (!hostChild || hostChild.pid == null) {
    return { active: false, error: "未找到当前窗口的 Host 进程" };
  }

  try {
    const port = await pickFreePort();
    const lanHost = pickLanIpv4();
    if (!lanHost) {
      return { active: false, error: "未找到可用的局域网 IPv4 地址" };
    }
    const token = randomBytes(24).toString("base64url");
    const link = `http://${lanHost}:${port}/?token=${encodeURIComponent(token)}`;
    const requestId = randomBytes(8).toString("hex");

    hostChild.postMessage({
      type: HostMessageTypes.WebRemoteControlStart,
      requestId,
      port,
      token,
      staticRoot: ensurePlaceholderStaticRoot(options.logger),
    });
    const ack = await waitForHostAck(hostChild, requestId, 10_000);
    if (!ack.ok) {
      options.logger.warn("[web-remote] Host 启动服务失败", { error: ack.error, port });
      return { active: false, error: ack.error ?? "服务启动失败" };
    }

    const state: WebRemoteControlSessionState = { active: true, url: link, link };
    sessionsByWebContentsId.set(senderId, { state, hostChild });
    options.logger.info("[web-remote] 服务已开启", { link, senderId });
    return state;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    options.logger.warn("[web-remote] 启动失败", { error: message });
    return { active: false, error: message };
  }
}

function stopSession(
  senderId: number,
  options: WebRemoteControlSessionOptions,
): WebRemoteControlSessionState {
  const session = sessionsByWebContentsId.get(senderId);
  if (!session) return IDLE_STATE;
  sessionsByWebContentsId.delete(senderId);
  try {
    session.hostChild.postMessage({ type: HostMessageTypes.WebRemoteControlStop });
  } catch (error) {
    options.logger.warn("[web-remote] 停止消息发送失败", {
      error: error instanceof Error ? error.message : String(error),
    });
  }
  options.logger.info("[web-remote] 服务已停止", { senderId });
  return IDLE_STATE;
}

export function registerWebRemoteControlIpcHandlers(options: {
  ipcMain: typeof ipcMain;
  windowHostProcessMap: Map<number, ElectronUtilityProcess>;
  logger: WebRemoteControlLogger;
}): void {
  options.ipcMain.handle(PlatformChannels.WebRemoteControlStartSession, async (event) =>
    startSession(event.sender.id, options),
  );
  options.ipcMain.handle(PlatformChannels.WebRemoteControlStopSession, (event) =>
    stopSession(event.sender.id, options),
  );
  options.ipcMain.handle(PlatformChannels.WebRemoteControlGetSessionState, (event) =>
    sessionsByWebContentsId.get(event.sender.id)?.state ?? IDLE_STATE,
  );
}

/** 窗口关闭时顺手清会话记录（Host 随窗口退出，监听自然消亡） */
export function forgetWebRemoteControlSession(senderId: number): void {
  sessionsByWebContentsId.delete(senderId);
}
