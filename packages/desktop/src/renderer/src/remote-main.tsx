/**
 * Web 远程控制手机端入口：把桌面同款 Root（完整对话区组件树）挂到
 * `web-remote-replayable` WebSocket 通道上——connectViaWebSocket 返回的
 * IServiceAccessor 与桌面 MessagePort 连接同形，Root 全量组件原样复用。
 *
 * 官方预留证据：Root.tsx 注释明示"Web/mobile 即使能看到权威状态也不装 reporter"，
 * RootProps 的 preferDirectoryBrowser / allowRemoteWorkspace /
 * initialWorkspaceLoadingFallback 都是给"Web 普通模式"留的口。
 */
import { createRoot } from "react-dom/client";
import { connectViaWebSocket } from "@zcode/client";
import { AppErrorBoundary, Root, ZCodeIntlProvider } from "@zcode/ui";
import "@zcode/ui/styles.css";
import { createWebRemotePlatform } from "./remotePlatform.js";

const platform = createWebRemotePlatform();
const prefersZh = navigator.language.toLowerCase().startsWith("zh");

function renderBoot(text: string): void {
  const boot = document.getElementById("boot");
  if (boot) boot.textContent = text;
}

async function boot(): Promise<void> {
  // 工作区路径由 Host 的 /api/server-info 透出（main 开服务时从弹窗 prop 透传）
  const info = (await fetch("/api/server-info").then((response) =>
    response.json(),
  )) as { workspaces?: Array<{ path?: string }> };
  const workspacePath = info.workspaces?.[0]?.path;
  if (!workspacePath) {
    throw new Error("server-info 缺少 workspaces（服务开太久或 Host 未就绪，重启服务再试）");
  }

  // ?token= 跟在页面 URL 上：静态页本身不拦，/ws 升级时中间件凭它种会话 cookie
  const wsProtocol = location.protocol === "https:" ? "wss://" : "ws://";
  const services = await connectViaWebSocket(
    `${wsProtocol}${location.host}/ws${location.search}`,
    {
      onClose: () => {
        renderBoot("与电脑的连接已断开（服务可能已停止），回桌面重新开服务后再扫。");
      },
    },
  );

  const root = createRoot(document.getElementById("root")!);
  root.render(
    <AppErrorBoundary isDesktop={false}>
      <ZCodeIntlProvider initialLocale={prefersZh ? "zh-CN" : "en-US"}>
        <Root
          services={services}
          platform={platform}
          isDesktop={false}
          presentationMode="mobile"
          restoreSession={false}
          supportsSettings={false}
          allowOpenWorkspace={false}
          allowRemoteWorkspace={false}
          preferDirectoryBrowser
          initialWorkspaceAbsPath={workspacePath}
          initialWorkspacePurpose="project"
          initialWorkspaceLoadingFallback={
            <div className="boot-hint">正在打开工作区…</div>
          }
        />
      </ZCodeIntlProvider>
    </AppErrorBoundary>,
  );
  // Root 挂载后撤掉启动壳（html 里的 #boot 由 body class 控制隐藏）
  document.body.classList.add("zcode-remote-ready");
}

boot().catch((error: unknown) => {
  renderBoot(
    `连接失败：${error instanceof Error ? error.message : String(error)}`,
  );
});
