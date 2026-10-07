import { useCallback, useEffect, useState } from "react";
import { ChevronLeft, RefreshCw } from "lucide-react";
import type { IServiceAccessor } from "@zcode/services";
import type { ZCodeSessionInfo } from "@zcode/shared";
import { cn } from "@/components/lib/utils.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { V4ChatPane } from "@/v4/V4ChatPane.js";

/**
 * 移动端远控壳（对照官方 mobileShell/mobileHome 信息架构）：
 * 首页 = 连接状态 + 当前工作区任务列表；点任务 → 全屏任务会话（桌面同款 V4ChatPane，
 * 流式/工具卡/Markdown 与桌面完全同源）。桌面三栏壳层（侧栏/页签/标题栏）在移动端不渲染。
 *
 * ponytail: 连接状态徽标 v1 静态展示"已连接"（能进本壳即 WS 已握手）；会话列表 20s
 * 轮询 + 手动刷新，不做订阅级实时——任务条目的流式状态在会话页内由桌面同款组件实时呈现，
 * 列表级实时（标题/状态徽标跳动）留待订阅通道接入后替换轮询。
 */

type MobileView = { kind: "home" } | { kind: "chat"; sessionId: string; title: string };

interface MobileRemoteShellProps {
  services: IServiceAccessor;
  workspacePath: string;
}

function formatTaskTime(timestamp: number, now: number): string {
  const date = new Date(timestamp);
  const sameDay = date.toDateString() === new Date(now).toDateString();
  if (sameDay) {
    return `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
  }
  const sameYear = date.getFullYear() === new Date(now).getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  if (sameYear) return `${month}-${day}`;
  return `${date.getFullYear()}-${month}-${day}`;
}

const taskStatusDotClassName: Record<ZCodeSessionInfo["status"], string> = {
  running: "bg-emerald-500 animate-pulse",
  waiting: "bg-amber-500",
  paused: "bg-amber-500/70",
  idle: "bg-muted-foreground/40",
  completed: "bg-muted-foreground/40",
  error: "bg-red-500",
};

export function MobileRemoteShell({ services, workspacePath }: MobileRemoteShellProps) {
  const { intl } = useZCodeIntl();
  const [view, setView] = useState<MobileView>({ kind: "home" });
  const [sessions, setSessions] = useState<ZCodeSessionInfo[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const [refreshing, setRefreshing] = useState(false);

  const loadSessions = useCallback(() => {
    setRefreshing(true);
    services.zcodeSessionService
      .listSessions({ workspacePath, limit: 100 })
      .then((items) => {
        setSessions(items.filter((item) => !item.workOrderOnly));
        setLoadError(null);
      })
      .catch((error) => {
        setLoadError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => {
        setRefreshing(false);
      });
  }, [services, workspacePath]);

  useEffect(() => {
    loadSessions();
    // 列表级准实时：手机锁屏/切后台回来时拉一次，常驻轻轮询兜底
    const timer = setInterval(loadSessions, 20_000);
    const onVisible = () => {
      if (document.visibilityState === "visible") loadSessions();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [loadSessions, reloadToken]);

  if (view.kind === "chat") {
    return (
      <div className="flex h-full flex-col bg-background">
        <header className="flex h-12 shrink-0 items-center gap-1 border-b border-border px-2">
          <button
            type="button"
            onClick={() => setView({ kind: "home" })}
            className="flex items-center gap-0.5 rounded-md px-2 py-1.5 text-sm text-muted-foreground active:bg-muted"
          >
            <ChevronLeft className="h-5 w-5" />
            {intl.formatMessage({ id: "webRemoteControl.mobileShell.backHome" })}
          </button>
          <span className="min-w-0 flex-1 truncate text-center text-sm font-medium text-foreground">
            {view.title || intl.formatMessage({ id: "webRemoteControl.mobileShell.chatTitle" })}
          </span>
          <span className="w-16 shrink-0" aria-hidden />
        </header>
        <div className="min-h-0 flex-1">
          <V4ChatPane
            key={view.sessionId}
            workspacePath={workspacePath}
            sessionId={view.sessionId}
            isDesktop={false}
          />
        </div>
      </div>
    );
  }

  const taskCount = sessions?.length ?? 0;

  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <header className="shrink-0 border-b border-border px-4 pb-3 pt-5">
        <div className="flex items-center justify-between">
          <h1 className="text-lg font-semibold">
            {intl.formatMessage({ id: "webRemoteControl.mobileHome.title" })}
          </h1>
          <span className="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-2.5 py-1 text-xs text-emerald-600 dark:text-emerald-400">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
            {intl.formatMessage({ id: "webRemoteControl.mobileHome.connected" })}
          </span>
        </div>
        <p className="mt-1.5 text-xs leading-relaxed text-muted-foreground">
          {intl.formatMessage({ id: "webRemoteControl.mobileHome.notice" })}
        </p>
      </header>

      <div className="flex items-center justify-between px-4 pb-2 pt-4">
        <div className="min-w-0">
          <h2 className="text-sm font-medium">
            {intl.formatMessage({ id: "webRemoteControl.mobileHome.sectionTitle" })}
          </h2>
          <p className="mt-0.5 text-xs text-muted-foreground">
            {intl.formatMessage(
              { id: "webRemoteControl.mobileHome.summary" },
              { workspaceCount: 1, taskCount },
            )}
          </p>
        </div>
        <button
          type="button"
          onClick={() => setReloadToken((token) => token + 1)}
          className="flex shrink-0 items-center gap-1 rounded-md border border-border px-2.5 py-1.5 text-xs text-muted-foreground active:bg-muted"
          aria-label={intl.formatMessage({ id: "webRemoteControl.mobileHome.refresh" })}
        >
          <RefreshCw className={cn("h-3.5 w-3.5", refreshing && "animate-spin")} />
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-[max(env(safe-area-inset-bottom),0.5rem)]">
        {loadError ? (
          <div className="mx-2 mt-3 rounded-lg border border-destructive/20 bg-destructive/5 p-3 text-sm text-destructive">
            <p className="break-all">{loadError}</p>
            <button
              type="button"
              onClick={() => setReloadToken((token) => token + 1)}
              className="mt-2 rounded-md border border-destructive/30 px-3 py-1 text-xs active:bg-destructive/10"
            >
              {intl.formatMessage({ id: "webRemoteControl.mobileHome.reconnect" })}
            </button>
          </div>
        ) : sessions === null ? (
          <div className="space-y-2 px-2 pt-3">
            {[0, 1, 2].map((index) => (
              <div key={index} className="h-14 animate-pulse rounded-lg bg-muted" />
            ))}
          </div>
        ) : sessions.length === 0 ? (
          <p className="px-4 pt-10 text-center text-sm text-muted-foreground">
            {intl.formatMessage({ id: "webRemoteControl.mobileHome.workspaceEmpty" })}
          </p>
        ) : (
          <ul className="space-y-1 px-2 pt-1">
            {sessions.map((task) => (
              <li key={task.sessionId}>
                <button
                  type="button"
                  onClick={() =>
                    setView({ kind: "chat", sessionId: task.sessionId, title: task.title })
                  }
                  className="flex w-full items-center gap-3 rounded-lg px-3 py-3 text-left active:bg-muted"
                >
                  <span
                    className={cn(
                      "h-2 w-2 shrink-0 rounded-full",
                      taskStatusDotClassName[task.status],
                    )}
                    aria-hidden
                  />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm text-foreground">
                      {task.title || intl.formatMessage({ id: "taskList.untitled" })}
                    </span>
                    <span className="mt-0.5 block text-xs text-muted-foreground">
                      {intl.formatMessage(
                        { id: "webRemoteControl.mobileHome.updatedAt" },
                        { time: formatTaskTime(task.updatedAt, Date.now()) },
                      )}
                    </span>
                  </span>
                  <ChevronLeft className="h-4 w-4 shrink-0 rotate-180 text-muted-foreground/50" />
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
