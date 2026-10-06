import type { WebRemoteControlSessionState } from "@zcode/shared";
import { HostMessageTypes, HostResponseTypes, PlatformChannels } from "@zcode/shared";
import { app, ipcMain, type UtilityProcess } from "electron";
import { randomBytes } from "node:crypto";
import { copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { networkInterfaces } from "node:os";
import { join, basename } from "node:path";
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

/** 把 marked/DOMPurify 的浏览器版复制进静态根（缺了页面自动退化纯文本，不阻断）。
 *  ponytail: 打包态的 node_modules 布局未验证，真机打包验收时再看要不要随包资源化。 */
function copyStaticLibs(root: string, logger: WebRemoteControlLogger): void {
  const libs: Array<{ source: string; dest: string }> = [];
  try {
    const require = createRequire(import.meta.url);
    libs.push(
      { source: require.resolve("marked/lib/marked.umd.js"), dest: "marked.umd.js" },
      { source: require.resolve("dompurify/dist/purify.min.js"), dest: "purify.min.js" },
    );
  } catch (error) {
    logger.warn("[web-remote] 静态库解析失败，手机页将退化纯文本", {
      error: error instanceof Error ? error.message : String(error),
    });
    return;
  }
  for (const lib of libs) {
    try {
      copyFileSync(lib.source, join(root, lib.dest));
    } catch (error) {
      logger.warn("[web-remote] 静态库复制失败", {
        dest: lib.dest,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }
}

/** 批 2 手机控制台（自包含单页，无构建步骤）：会话列表 + 消息流（轮询）+ 发言。
 *  助手消息走 marked+DOMPurify 的 Markdown 渲染（库缺失退化纯文本）；
 *  流式订阅后续走 /ws 的 RPC 通道；本页只吃三条 token 保护的 /api/remote/* 路由。 */
function renderPlaceholderHtml(): string {
  return `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>ZCode 工作区</title>
<script src="/marked.umd.js"></script>
<script src="/purify.min.js"></script>
<style>
  :root { color-scheme: dark; --bg: #26221e; --panel: #322d28; --line: #3a352f;
          --ink: #e8e4de; --sub: #a89f92; --accent: #c26736; --code-bg: #1d1a17; }
  * { box-sizing: border-box; }
  body { font-family: system-ui, -apple-system, sans-serif; margin: 0; background: var(--bg); color: var(--ink);
         display: flex; flex-direction: column; height: 100dvh; }
  header { padding: calc(10px + env(safe-area-inset-top)) 14px 10px; font-weight: 600; font-size: 15px;
           border-bottom: 1px solid var(--line); display: flex; align-items: center; gap: 8px; }
  header select { flex: 1; min-width: 0; background: var(--panel); color: inherit; border: 1px solid #4a443c;
                  border-radius: 8px; padding: 6px 8px; font-size: 14px; }
  #msgs { flex: 1; overflow-y: auto; padding: 14px 12px 6px; display: flex; flex-direction: column; gap: 14px; }
  /* 助手消息：全宽平铺 Markdown（对齐桌面观感）；用户消息：右对齐气泡 */
  .assistant { width: 100%; font-size: 14.5px; line-height: 1.6; }
  .user { align-self: flex-end; max-width: 86%; background: #7c5c3e; color: #fdf9f3; padding: 8px 12px;
          border-radius: 12px; border-bottom-right-radius: 4px; font-size: 14.5px; line-height: 1.55;
          white-space: pre-wrap; word-break: break-word; }
  .meta { align-self: center; font-size: 11.5px; color: var(--sub); padding: 2px 0; }
  .hint { text-align: center; color: var(--sub); font-size: 13px; padding: 24px; }
  footer { padding: 8px 10px calc(10px + env(safe-area-inset-bottom)); border-top: 1px solid var(--line);
           display: flex; gap: 8px; }
  #input { flex: 1; min-width: 0; background: var(--panel); color: inherit; border: 1px solid #4a443c;
           border-radius: 10px; padding: 10px 12px; font-size: 16px; }
  #send { background: var(--accent); color: #fff; border: 0; border-radius: 10px; padding: 0 16px;
          font-size: 15px; font-weight: 600; }
  #send:disabled { opacity: .5; }
  /* ---- Markdown 排版（对齐桌面暖色暗主题）---- */
  .assistant > :first-child { margin-top: 0; }
  .assistant > :last-child { margin-bottom: 0; }
  .assistant p { margin: .5em 0; }
  .assistant h1, .assistant h2, .assistant h3, .assistant h4 { margin: 1em 0 .4em; line-height: 1.35; }
  .assistant h1 { font-size: 1.25em; } .assistant h2 { font-size: 1.15em; } .assistant h3 { font-size: 1.05em; }
  .assistant ul, .assistant ol { margin: .5em 0; padding-left: 1.4em; }
  .assistant li { margin: .25em 0; }
  .assistant code { font-family: ui-monospace, Consolas, monospace; font-size: .88em; background: var(--code-bg);
                    border: 1px solid #37312b; border-radius: 5px; padding: .1em .35em; }
  .assistant pre { background: var(--code-bg); border: 1px solid #37312b; border-radius: 10px; padding: 10px 12px;
                   overflow-x: auto; margin: .6em 0; }
  .assistant pre code { background: none; border: 0; padding: 0; font-size: .85em; line-height: 1.5; white-space: pre; }
  .assistant blockquote { margin: .6em 0; padding: .1em 0 .1em 12px; border-left: 3px solid #4a443c; color: var(--sub); }
  .assistant a { color: #e0985f; text-decoration: underline; text-underline-offset: 2px; }
  .assistant table { border-collapse: collapse; margin: .6em 0; font-size: .92em; display: block; overflow-x: auto; }
  .assistant th, .assistant td { border: 1px solid #4a443c; padding: 5px 9px; }
  .assistant hr { border: 0; border-top: 1px solid var(--line); margin: 1em 0; }
</style></head><body>
<header>ZCode<span style="flex:1"></span><select id="sessions"></select></header>
<div id="msgs"><div class="hint">正在连接工作区…</div></div>
<footer>
  <input id="input" type="text" placeholder="发消息给工作区…" autocomplete="off">
  <button id="send">发送</button>
</footer>
<script>
(() => {
  const canRenderMarkdown = Boolean(window.marked && window.DOMPurify);
  function renderAssistant(text) {
    const el = document.createElement("div");
    el.className = "assistant";
    if (canRenderMarkdown) {
      // marked 输出经 DOMPurify 白名单消毒再上屏（模型输出里也可能藏 HTML）
      el.innerHTML = DOMPurify.sanitize(marked.parse(text, { breaks: true, gfm: true }));
    } else {
      el.textContent = text;
    }
    return el;
  }
  const qs = (o) => new URLSearchParams(o).toString();
  const msgsEl = document.getElementById("msgs");
  const selEl = document.getElementById("sessions");
  const inputEl = document.getElementById("input");
  const sendEl = document.getElementById("send");
  let workspacePath = null;
  let sessionId = sessionStorage.getItem("remoteSessionId") || null;
  let sendBusy = false;

  async function jfetch(url, options) {
    const res = await fetch(url, options);
    const body = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(body.error || ("HTTP " + res.status));
    return body;
  }

  function esc(text) {
    return text.replace(/[&<>"]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[ch]));
  }

  function renderMessages(messages) {
    if (!messages.length) { msgsEl.innerHTML = '<div class="hint">这个会话还没有消息</div>'; return; }
    msgsEl.innerHTML = "";
    let last = null;
    for (const m of messages) {
      const role = m.info && m.info.role === "user" ? "user" : "assistant";
      const text = (m.parts || []).map((p) => (p.type === "text" ? p.text || "" : "")).join("").trim();
      if (!text) continue;
      if (text.startsWith("<system-reminder>") || text.startsWith("<system-reminder ")) continue;
      last = role;
      if (role === "user") {
        msgsEl.insertAdjacentHTML("beforeend", '<div class="user">' + esc(text) + "</div>");
      } else {
        msgsEl.appendChild(renderAssistant(text));
      }
    }
    if (!last) { msgsEl.innerHTML = '<div class="hint">（没有可显示的文本消息）</div>'; return; }
    msgsEl.scrollTop = msgsEl.scrollHeight;
  }

  async function loadMessages() {
    if (!workspacePath || !sessionId) return;
    try {
      const body = await jfetch("/api/remote/messages?" + qs({ workspacePath, sessionId }));
      renderMessages(body.messages || []);
      msgsEl.dataset.state = "ok";
    } catch (e) {
      if (msgsEl.dataset.state !== "error") {
        msgsEl.dataset.state = "error";
        msgsEl.innerHTML = '<div class="hint">加载消息失败：' + esc(e.message) + "<br>每 2 秒自动重试，成功即恢复</div>";
      }
    }
  }

  async function loadSessions() {
    const body = await jfetch("/api/remote/sessions?" + qs({ workspacePath }));
    const sessions = (body.sessions || []).filter((s) => !s.workOrderOnly);
    if (!sessions.length) { msgsEl.innerHTML = '<div class="hint">这个工作区还没有会话，回桌面先聊一句</div>'; return; }
    const current = sessionId && sessions.some((s) => s.sessionId === sessionId)
      ? sessionId : sessions[0].sessionId;
    if (current !== sessionId) { sessionId = current; sessionStorage.setItem("remoteSessionId", current); }
    selEl.innerHTML = sessions.map((s) =>
      '<option value="' + esc(s.sessionId) + '"' + (s.sessionId === current ? " selected" : "") + ">" +
      esc(s.title || "未命名会话") + "</option>").join("");
    await loadMessages();
  }

  selEl.addEventListener("change", () => {
    sessionId = selEl.value; sessionStorage.setItem("remoteSessionId", sessionId); void loadMessages();
  });

  async function send() {
    const content = inputEl.value.trim();
    if (!content || !workspacePath || !sessionId || sendBusy) return;
    sendBusy = true; sendEl.disabled = true;
    msgsEl.insertAdjacentHTML("beforeend", '<div class="user">' + esc(content) + "</div>");
    msgsEl.scrollTop = msgsEl.scrollHeight;
    inputEl.value = "";
    try {
      await jfetch("/api/remote/send", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ workspacePath, sessionId, content }),
      });
      setTimeout(() => { void loadMessages(); }, 800);
    } catch (e) {
      msgsEl.insertAdjacentHTML("beforeend", '<div class="meta">发送失败：' + esc(e.message) + "</div>");
    } finally { sendBusy = false; sendEl.disabled = false; inputEl.focus(); }
  }
  sendEl.addEventListener("click", () => void send());
  inputEl.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.isComposing) { e.preventDefault(); void send(); }
  });

  (async () => {
    try {
      const info = await jfetch("/api/server-info");
      workspacePath = info.workspaces && info.workspaces[0] && info.workspaces[0].path;
      if (!workspacePath) throw new Error("server-info 缺少 workspaces");
      await loadSessions();
      window.setInterval(() => { void loadMessages(); }, 2000);
      window.setInterval(() => { void loadSessions().catch(() => undefined); }, 8000);
    } catch (e) {
      msgsEl.innerHTML = '<div class="hint">连接失败：' + esc(e.message) + "</div>";
    }
  })();
})();
</script></body></html>`;
}

/** 批 2 手机控制台静态根：写 index.html + 复制 Markdown 渲染库 */
function ensurePlaceholderStaticRoot(logger: WebRemoteControlLogger): string {
  const root = join(app.getPath("userData"), "web-remote-control", "static");
  try {
    mkdirSync(root, { recursive: true });
    writeFileSync(join(root, "index.html"), renderPlaceholderHtml(), "utf8");
    copyStaticLibs(root, logger);
  } catch (error) {
    logger.warn("[web-remote] 手机控制台页写入失败", {
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
  request: { workspacePath?: string },
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
    const workspacePath = request.workspacePath?.trim() || undefined;
    const workspaces = workspacePath
      ? [{ path: workspacePath, label: basename(workspacePath) || workspacePath }]
      : undefined;

    hostChild.postMessage({
      type: HostMessageTypes.WebRemoteControlStart,
      requestId,
      port,
      token,
      staticRoot: ensurePlaceholderStaticRoot(options.logger),
      ...(workspaces ? { workspaces } : {}),
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
  options.ipcMain.handle(
    PlatformChannels.WebRemoteControlStartSession,
    async (event, request?: { workspacePath?: string }) =>
      startSession(event.sender.id, request ?? {}, options),
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
