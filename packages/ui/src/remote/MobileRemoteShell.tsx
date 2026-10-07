import { useCallback, useEffect, useMemo, useRef, useState, Component, type ReactNode } from "react";
import {
  Bot,
  Check,
  ChevronLeft,
  ChevronRight,
  ChevronsDownUp,
  ChevronsUpDown,
  Clock,
  Ellipsis,
  Folder,
  MessageSquarePlus,
  Palette,
  Plus,
  RefreshCw,
  SlidersHorizontal,
  X,
} from "lucide-react";
import type { IServiceAccessor } from "@zcode/services";
import type { AppSettings, WorkspacePurpose } from "@zcode/shared";
import { cn } from "@/components/lib/utils.js";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu.js";
import { toast } from "@/components/ui/toast.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import type { CodeViewerSource } from "@/lib/codeViewer.js";
import { inferCodeLanguage } from "@/lib/codeViewer.js";
import { resolveSubagentColorFromName, SUBAGENT_COLOR_CLASS } from "@/lib/subagentColors.js";
import type { WorkspaceTabState } from "@/store/tabStore.js";
import { useWorkspaceTaskLists } from "@/hooks/useWorkspaceTaskLists.js";
import { getPlainTextPatchFallbackLines } from "@/lib/patchDiffPreview.js";
import { HighlightedLightweightDiffPreview } from "@/components/ui/highlighted-lightweight-diff-preview.js";
import { useZCodeStore } from "@/store/StoreProvider.js";
import { applyTaskQueryCacheMutation } from "@/store/taskQueryCacheStore.js";
import { useConfirmDialog } from "@/hooks/useConfirmDialog.js";
import { useTaskListItemContextActions } from "@/useTaskListItemContextActions.js";
import { TaskActionMenuContent } from "@/TaskActionMenuContent.js";
import { TaskRenameDialog } from "@/TaskRenameDialog.js";
import { resolveTheme } from "@/useTheme.js";
import { useWorkspaceProjectAgents } from "@/WorkspaceSidebar/ProjectAgents.js";
import {
  applyDerivedPersonaChatBadges,
  getPersonaChatBadge,
  restorePersonaTitlePrefix,
  stripPersonaTitlePrefix,
} from "@/WorkspaceSidebar/projectAgentsModel.js";
import { PreviewPane } from "@/PreviewPane.js";
import { V4ChatPane } from "@/v4/V4ChatPane.js";

/**
 * 移动端远控壳（对照官方 mobileShell/mobileHome 信息架构）：
 * 首页 = 连接状态 + 当前设备上的工作区和任务卡片（可收纳；官方卡片=图标+名称+
 * 本地/对话徽标+路径+任务数+新建"+"+更新时间）。任务列表走桌面侧栏同款
 * useWorkspaceTaskLists 投影（合议轮/评审轮/工单/归档的进出与桌面完全一致，
 * 任务数与桌面侧栏对得上）；点任务 → 全屏任务会话（桌面同款 V4ChatPane）；
 * 卡片"+"= 新建任务（draft 会话）。桌面三栏壳层在移动端不渲染。
 *
 * ponytail: 连接状态徽标 v1 静态展示"已连接"；官方"整理任务（按时间线/按工作区）
 * /排序方式"开关 v1 不做（固定按工作区分组、更新时间倒序、每卡 20 条）——缺口
 * 出现再加；文件审查/查看走全屏 PreviewPane 浮层（手机形态），桌面的壳层右坞
 * （终端/embedded browser 等）不在移动端复刻。
 */

type MobileView =
  | { kind: "home" }
  | {
      kind: "chat";
      /** key 稳定符：draft 首发建会话后不得变，否则整树重挂 */
      entryToken: number;
      workspacePath: string;
      sessionId: string | null;
      title: string;
    };

interface WorkspaceCard {
  workspacePath: string;
  label: string;
  purpose?: WorkspacePurpose;
  identity?: string;
}

interface MobileRemoteShellProps {
  services: IServiceAccessor;
  workspacePath: string;
}

function workspaceLabel(path: string): string {
  return path.split(/[\\/]+/).filter(Boolean).pop() ?? path;
}

/** 官方口径的相对时间：3分 / 2天 / 1天 */
function formatRelativeTime(timestamp: number, now: number): string {
  const minutes = Math.floor((now - timestamp) / 60_000);
  if (minutes < 1) return "刚刚";
  if (minutes < 60) return `${minutes}分`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}小时`;
  return `${Math.floor(hours / 24)}天`;
}

const TASK_STATUS_DOT_CLASS: Record<string, string> = {
  running: "bg-emerald-500 animate-pulse",
  error: "bg-red-500",
  completed: "bg-muted-foreground/40",
};

/** 设备上打开过的工作区（桌面恢复会话同一来源）：local 项目 + 对话工作区，去重限量。 */
function resolveWorkspaceCards(
  settings: AppSettings,
  fallbackPath: string,
  conversationLabel: string,
): WorkspaceCard[] {
  const seen = new Set<string>();
  const cards: WorkspaceCard[] = [];
  const push = (path: string, purpose?: WorkspacePurpose) => {
    if (!path || seen.has(path)) return;
    seen.add(path);
    cards.push({
      workspacePath: path,
      label: purpose === "conversation" ? conversationLabel : workspaceLabel(path),
      purpose,
    });
  };
  for (const entry of settings.lastWorkspaceSession ?? []) {
    if (entry.kind !== "local") continue;
    push(entry.workspacePath, entry.workspacePurpose);
  }
  // 当前窗口工作区兜底（可能尚未落进 lastWorkspaceSession）
  push(fallbackPath);
  return cards.slice(0, 10);
}

const MOBILE_TASK_VISIBLE_LIMIT = 20;

/** 查看器崩了不能连坐聊天：就地显示异常（也是定位渲染问题的窗口）。 */
class MobileViewerErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  render() {
    if (this.state.error) {
      return (
        <div className="h-full overflow-auto p-4">
          <p className="text-sm text-destructive break-all">
            {this.state.error.message || String(this.state.error)}
          </p>
          <pre className="mt-2 whitespace-pre-wrap break-all text-[10px] leading-3 text-muted-foreground">
            {this.state.error.stack ?? ""}
          </pre>
        </div>
      );
    }
    return this.props.children;
  }
}

export function MobileRemoteShell({ services, workspacePath }: MobileRemoteShellProps) {
  const { intl } = useZCodeIntl();
  const [view, setView] = useState<MobileView>({ kind: "home" });
  const [cards, setCards] = useState<WorkspaceCard[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const [refreshing, setRefreshing] = useState(false);
  const [expandedKeys, setExpandedKeys] = useState<Set<string>>(new Set());
  const [allExpanded, setAllExpanded] = useState(false);
  const entryTokenRef = useRef(0);

  const conversationLabel = intl.formatMessage({
    id: "webRemoteControl.mobileHome.conversationLabel",
  });

  const loadCards = useCallback(() => {
    setRefreshing(true);
    services.settingService
      .get()
      .then((settings) => {
        setCards(resolveWorkspaceCards(settings, workspacePath, conversationLabel));
        setLoadError(null);
      })
      .catch((error) => {
        setLoadError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => {
        setRefreshing(false);
      });
  }, [services, workspacePath, conversationLabel]);

  useEffect(() => {
    loadCards();
    const onVisible = () => {
      if (document.visibilityState === "visible") loadCards();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [loadCards, reloadToken]);

  // 官方 mobileHome 的"整理任务/排序方式"（sliders 菜单）——先于任务投影声明
  const [organizeMode, setOrganizeMode] = useState<"workspace" | "timeline">("workspace");
  const [sortMode, setSortMode] = useState<"created" | "updated">("updated");

  // 桌面侧栏同款任务投影：同一钩子、同一分页、同一排序——任务数与桌面侧栏一致
  //（合议轮/评审轮/工单子会话/归档的进出全部由投影决定，不在此处再筛）。
  const workspaceTabs = useMemo<WorkspaceTabState[]>(
    () =>
      (cards ?? [{ workspacePath, label: workspaceLabel(workspacePath) }]).map((card, index) => ({
        kind: "workspace" as const,
        id: `mobile-ws-${index}`,
        workspacePath: card.workspacePath,
        label: card.label,
        ...(card.purpose ? { workspacePurpose: card.purpose } : {}),
      })),
    [cards, workspacePath],
  );

  const workspaceTaskLists = useWorkspaceTaskLists({
    workspaceTabs,
    activeWorkspacePath: workspacePath,
    sortBy: sortMode,
    visibleLimitByWorkspaceKey: {},
    defaultVisibleLimit: MOBILE_TASK_VISIBLE_LIMIT,
  });

  const cardByWorkspaceKey = useMemo(() => {
    const map = new Map<string, WorkspaceCard>();
    for (const card of cards ?? []) map.set(card.workspacePath, card);
    return map;
  }, [cards]);

  // 员工名册（桌面侧栏同款）：任务行工牌（彩色名字牌）的反推数据源
  const projectAgents = useWorkspaceProjectAgents({
    tabs: workspaceTabs,
    boundWorkspacePath: workspacePath,
  });

  // 文件审查/查看浮层：onOpenCodeViewer 落到这里（手机形态=右抽屉，不是桌面右坞）
  const [viewerSource, setViewerSource] = useState<CodeViewerSource | null>(null);
  const theme = useZCodeStore((state) => state.theme);
  const setTheme = useZCodeStore((state) => state.setTheme);
  const codePreviewSettings = useZCodeStore((state) => state.codePreviewSettings);
  // 会话页任务菜单（"…"）：重命名弹窗状态
  const [renameOpen, setRenameOpen] = useState(false);
  const [renameDraft, setRenameDraft] = useState("");
  const renameInputRef = useRef<HTMLInputElement | null>(null);
  const confirmDialog = useConfirmDialog();
  // "…" 菜单的任务级上下文（会话文件/日志路径、文件管理器文案）——仅会话页装载
  const chatTaskId = view.kind === "chat" ? (view.sessionId ?? "") : "";
  const taskContext = useTaskListItemContextActions({
    workspacePath: view.kind === "chat" ? view.workspacePath : workspacePath,
    taskId: chatTaskId,
    intl,
    loadTaskPaths: view.kind === "chat" && view.sessionId !== null,
  });

  // 整理任务=按时间线：跨工作区拍平（排序方式随 sliders 菜单）
  const timelineItems = useMemo(() => {
    if (organizeMode !== "timeline") return null;
    const items = workspaceTaskLists.groups.flatMap((group) =>
      applyDerivedPersonaChatBadges(
        group.items,
        projectAgents.agentsByWorkspaceKey.get(group.workspacePath) ?? [],
      ).map((task) => ({
        task,
        workspacePath: group.workspacePath,
        workspaceLabel:
          cardByWorkspaceKey.get(group.workspacePath)?.label ??
          workspaceLabel(group.workspacePath),
      })),
    );
    items.sort((a, b) =>
      sortMode === "created"
        ? b.task.createdAt - a.task.createdAt
        : b.task.updatedAt - a.task.updatedAt,
    );
    return items;
  }, [organizeMode, sortMode, workspaceTaskLists.groups, projectAgents.agentsByWorkspaceKey, cardByWorkspaceKey]);

  const openChat = useCallback(
    (target: { workspacePath: string; sessionId: string | null; title: string }) => {
      entryTokenRef.current += 1;
      setView({
        kind: "chat",
        entryToken: entryTokenRef.current,
        workspacePath: target.workspacePath,
        sessionId: target.sessionId,
        title: target.title,
      });
    },
    [],
  );

  const toggleCard = useCallback((key: string) => {
    setExpandedKeys((previous) => {
      const next = new Set(previous);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  }, []);

  // 局域网 http 下 navigator.clipboard 不可用，剪贴板走 execCommand 兜底
  const copyText = useCallback(async (value: string | null, label?: string) => {
    if (!value) return;
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(value);
      } else {
        const ta = document.createElement("textarea");
        ta.value = value;
        ta.style.position = "fixed";
        ta.style.opacity = "0";
        document.body.appendChild(ta);
        ta.select();
        document.execCommand("copy");
        ta.remove();
      }
      toast(label ?? intl.formatMessage({ id: "startup.global.copied" }));
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error));
    }
  }, [intl]);

  // "…" 菜单动作（桌面 WorkspaceHeaderSections 同款服务链路；列表即时性靠轮询兜底）
  const handleTogglePinTask = useCallback(() => {
    if (view.kind !== "chat" || !view.sessionId) return;
    void services.zcodeTaskService
      .setTaskPinned({
        taskId: view.sessionId,
        workspacePath: view.workspacePath,
        pinned: true,
      })
      .then(() => {
        toast(intl.formatMessage({ id: "taskList.pinSucceed" }));
      })
      .catch((error) => {
        toast(error instanceof Error ? error.message : String(error));
      });
  }, [view, services, intl]);

  const handleStartRenameTask = useCallback(() => {
    if (view.kind !== "chat") return;
    setRenameDraft(view.title);
    setRenameOpen(true);
  }, [view]);

  const handleRenameConfirm = useCallback(() => {
    if (view.kind !== "chat" || !view.sessionId) return;
    const normalizedTitle = restorePersonaTitlePrefix(view.title, renameDraft.trim());
    void services.zcodeTaskService
      .renameTask({
        taskId: view.sessionId,
        workspacePath: view.workspacePath,
        title: normalizedTitle,
      })
      .then((renamedTask) => {
        setView((previous) =>
          previous.kind === "chat" ? { ...previous, title: renamedTask.title } : previous,
        );
        toast(intl.formatMessage({ id: "taskList.renameSucceed" }));
      })
      .catch((error) => {
        toast(error instanceof Error ? error.message : String(error));
      });
    setRenameOpen(false);
  }, [view, renameDraft, services, intl]);

  const handleArchiveTask = useCallback(async () => {
    if (view.kind !== "chat" || !view.sessionId) return;
    const confirmed = await confirmDialog({
      title: intl.formatMessage({ id: "confirmDialog.taskArchiveTitle" }),
      description: intl.formatMessage(
        { id: "confirmDialog.taskArchiveDescription" },
        { taskTitle: view.title || intl.formatMessage({ id: "taskList.untitled" }) },
      ),
      confirmLabel: intl.formatMessage({ id: "taskList.archive" }),
    });
    if (!confirmed) return;
    void services.zcodeTaskService
      .archiveTask({ taskId: view.sessionId, workspacePath: view.workspacePath })
      .then(() => {
        toast(intl.formatMessage({ id: "taskList.archiveSucceed" }));
        setViewerSource(null);
        setView({ kind: "home" });
        setReloadToken((token) => token + 1);
      })
      .catch((error) => {
        toast(error instanceof Error ? error.message : String(error));
      });
  }, [view, services, intl, confirmDialog]);

  const handleMarkTaskAsUnread = useCallback(() => {
    if (view.kind !== "chat" || !view.sessionId) return;
    void services.zcodeTaskService
      .setTaskUnread({
        taskId: view.sessionId,
        workspacePath: view.workspacePath,
        unread: true,
      })
      .then(() => {
        toast(intl.formatMessage({ id: "taskList.unreadSucceed" }));
        setView({ kind: "home" });
        setReloadToken((token) => token + 1);
      })
      .catch((error) => {
        toast(error instanceof Error ? error.message : String(error));
      });
  }, [view, services, intl]);

  if (view.kind === "chat") {
    return (
      <div className="relative flex h-full flex-col bg-background">
        {/* 第一行：官方 chrome 行（返回 + 页面标题 + 任务菜单 + 界面风格） */}
        <header className="flex h-12 shrink-0 items-center gap-1 border-b border-border px-2">
          <button
            type="button"
            onClick={() => {
              setViewerSource(null);
              setView({ kind: "home" });
            }}
            className="flex items-center gap-0.5 rounded-md px-2 py-1.5 text-sm text-muted-foreground active:bg-muted"
          >
            <ChevronLeft className="h-5 w-5" />
            {intl.formatMessage({ id: "webRemoteControl.mobileShell.chatTitle" })}
          </button>
          <span className="min-w-0 flex-1" />
          {view.sessionId ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button
                  type="button"
                  className="rounded-md p-1.5 text-muted-foreground active:bg-muted"
                  aria-label={intl.formatMessage({ id: "common.more" })}
                >
                  <Ellipsis className="h-5 w-5" />
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-52">
                <TaskActionMenuContent
                  intl={intl}
                  isPinned={false}
                  fileManagerLabel={taskContext.fileManagerLabel}
                  taskSessionFile={taskContext.taskSessionFile}
                  activeSessionId={view.sessionId}
                  taskNativeSessionLogFile={taskContext.taskNativeSessionLogFile}
                  hideMobileUnsupportedActions
                  Item={DropdownMenuItem}
                  Separator={DropdownMenuSeparator}
                  onTogglePinTask={handleTogglePinTask}
                  onStartRenameTask={handleStartRenameTask}
                  onArchiveTask={() => void handleArchiveTask()}
                  onMarkTaskAsUnread={handleMarkTaskAsUnread}
                  onOpenTaskPathInFileManager={() => {}}
                  onCopyWorkspacePath={() => void copyText(view.workspacePath)}
                  onCopyTaskPath={() =>
                    void copyText(taskContext.taskSessionFile.path)
                  }
                  onCopyTaskLogPath={() =>
                    void copyText(taskContext.taskNativeSessionLogFile.path)
                  }
                  onCopySessionId={() => void copyText(view.sessionId)}
                />
              </DropdownMenuContent>
            </DropdownMenu>
          ) : null}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                className="rounded-md p-1.5 text-muted-foreground active:bg-muted"
                aria-label={intl.formatMessage({ id: "settings.themeMode" })}
              >
                <Palette className="h-5 w-5" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-40">
              {(
                [
                  ["system", "settings.themeMode.system"],
                  ["zai-light", "settings.themeMode.light"],
                  ["zai-dark", "settings.themeMode.dark"],
                ] as const
              ).map(([value, labelKey]) => (
                <DropdownMenuItem key={value} onSelect={() => setTheme(value)}>
                  {intl.formatMessage({ id: labelKey })}
                  {theme === value ? <Check className="ml-auto h-4 w-4" /> : null}
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        </header>
        <div className="min-h-0 flex-1">
          <V4ChatPane
            key={view.entryToken}
            workspacePath={view.workspacePath}
            sessionId={view.sessionId}
            isDesktop={false}
            onSessionCreated={(sessionId) => {
              setView((previous) =>
                previous.kind === "chat"
                  ? { ...previous, sessionId, title: previous.title }
                  : previous,
              );
            }}
            onOpenCodeViewer={(source) => setViewerSource(source)}
          />
        </div>
        {/* 重命名任务弹窗（"…"菜单入口） */}
        <TaskRenameDialog
          open={renameOpen}
          value={renameDraft}
          inputRef={renameInputRef}
          intl={intl}
          onOpenChange={setRenameOpen}
          onChange={setRenameDraft}
          onCancel={() => setRenameOpen(false)}
          onConfirm={handleRenameConfirm}
        />
        {viewerSource ? (
          <>
            {/* 遮罩：点它关闭；会话保持挂载在后面（官方抽屉式右坞的移动形态） */}
            <div
              className="absolute inset-0 z-40 bg-black/40 animate-in fade-in-0 duration-200"
              onClick={() => setViewerSource(null)}
              aria-hidden
            />
            {/* 抽屉：右侧滑入占 ~94%，左侧留一条变暗的会话 */}
            <div className="absolute inset-y-0 right-0 z-50 flex w-[94%] flex-col border-l border-border bg-background shadow-2xl animate-in slide-in-from-right-[100%] fade-in-0 duration-200">
              <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border px-2">
                <button
                  type="button"
                  onClick={() => setViewerSource(null)}
                  className="rounded-md p-1.5 text-muted-foreground active:bg-muted"
                  aria-label={intl.formatMessage({ id: "common.close" })}
                >
                  <X className="h-5 w-5" />
                </button>
                <span className="min-w-0 flex-1 truncate text-sm font-medium text-foreground">
                  {viewerSource.title}
                </span>
              </header>
              <div className="min-h-0 flex-1">
                <MobileViewerErrorBoundary>
                  {viewerSource.type === "patch" ? (
                    // 手机抽屉绕开 @pierre/diffs 的 PatchDiff（移动端渲染会崩，桌面同源未查），
                    // 直接用会话代码块同款的轻量 Shiki diff——纯文本行渲染，移动端已验证可用。
                    <HighlightedLightweightDiffPreview
                      className="h-full"
                      codePreviewSettings={codePreviewSettings}
                      language={inferCodeLanguage(
                        viewerSource.path ?? viewerSource.title,
                        viewerSource.patch,
                      )}
                      lines={
                        getPlainTextPatchFallbackLines(viewerSource.patch) ??
                        viewerSource.patch.split(/\r?\n/)
                      }
                      path={viewerSource.path ?? viewerSource.title}
                      theme={
                        resolveTheme(theme) === "dark"
                          ? codePreviewSettings.darkTheme
                          : codePreviewSettings.lightTheme
                      }
                    />
                  ) : (
                    <PreviewPane
                      source={viewerSource}
                      onClose={() => setViewerSource(null)}
                      workspacePath={view.workspacePath}
                    />
                  )}
                </MobileViewerErrorBoundary>
              </div>
            </div>
          </>
        ) : null}
      </div>
    );
  }

  const groups = workspaceTaskLists.groups;
  const taskCount = groups.reduce((sum, group) => sum + group.total, 0);
  const workspaceCount = cards?.length ?? groups.length;

  const toggleAllCards = () => {
    if (allExpanded) {
      setExpandedKeys(new Set());
      setAllExpanded(false);
    } else {
      setExpandedKeys(
        new Set(
          groups.map((group) =>
            group.workspaceIdentity
              ? `${group.workspacePath}::${group.workspaceIdentity}`
              : group.workspacePath,
          ),
        ),
      );
      setAllExpanded(true);
    }
  };

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
              { workspaceCount, taskCount },
            )}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {/* 官方 sliders 菜单：整理任务（按工作区/按时间线）+ 排序方式（创建/更新） */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                className="rounded-md p-1.5 text-muted-foreground active:bg-muted"
                aria-label={intl.formatMessage({
                  id: "webRemoteControl.mobileHome.organize",
                })}
              >
                <SlidersHorizontal className="h-4 w-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-44">
              <DropdownMenuLabel>
                {intl.formatMessage({ id: "webRemoteControl.mobileHome.organize" })}
              </DropdownMenuLabel>
              <DropdownMenuItem onSelect={() => setOrganizeMode("workspace")}>
                <Folder className="h-4 w-4" />
                {intl.formatMessage({ id: "webRemoteControl.mobileHome.organizeByWorkspace" })}
                {organizeMode === "workspace" ? <Check className="ml-auto h-4 w-4" /> : null}
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => setOrganizeMode("timeline")}>
                <Clock className="h-4 w-4" />
                {intl.formatMessage({ id: "webRemoteControl.mobileHome.organizeByTimeline" })}
                {organizeMode === "timeline" ? <Check className="ml-auto h-4 w-4" /> : null}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuLabel>
                {intl.formatMessage({ id: "webRemoteControl.mobileHome.sortBy" })}
              </DropdownMenuLabel>
              <DropdownMenuItem onSelect={() => setSortMode("created")}>
                {intl.formatMessage({ id: "webRemoteControl.mobileHome.sortByCreated" })}
                {sortMode === "created" ? <Check className="ml-auto h-4 w-4" /> : null}
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => setSortMode("updated")}>
                {intl.formatMessage({ id: "webRemoteControl.mobileHome.sortByUpdated" })}
                {sortMode === "updated" ? <Check className="ml-auto h-4 w-4" /> : null}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          <button
            type="button"
            onClick={toggleAllCards}
            className="rounded-md p-1.5 text-muted-foreground active:bg-muted"
            aria-label={intl.formatMessage({
              id: allExpanded
                ? "webRemoteControl.mobileHome.collapseAll"
                : "webRemoteControl.mobileHome.expandAll",
            })}
          >
            {allExpanded ? (
              <ChevronsDownUp className="h-4 w-4" />
            ) : (
              <ChevronsUpDown className="h-4 w-4" />
            )}
          </button>
          <button
            type="button"
            onClick={() => setReloadToken((token) => token + 1)}
            className="rounded-md p-1.5 text-muted-foreground active:bg-muted"
            aria-label={intl.formatMessage({ id: "webRemoteControl.mobileHome.refresh" })}
          >
            <RefreshCw className={cn("h-4 w-4", refreshing && "animate-spin")} />
          </button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-3 pb-[max(env(safe-area-inset-bottom),0.5rem)]">
        {loadError ? (
          <div className="mt-3 rounded-lg border border-destructive/20 bg-destructive/5 p-3 text-sm text-destructive">
            <p className="break-all">{loadError}</p>
            <button
              type="button"
              onClick={() => setReloadToken((token) => token + 1)}
              className="mt-2 rounded-md border border-destructive/30 px-3 py-1 text-xs active:bg-destructive/10"
            >
              {intl.formatMessage({ id: "webRemoteControl.mobileHome.reconnect" })}
            </button>
          </div>
        ) : cards === null ? (
          <div className="space-y-2 pt-2">
            {[0, 1, 2].map((index) => (
              <div key={index} className="h-20 animate-pulse rounded-xl bg-muted" />
            ))}
          </div>
        ) : organizeMode === "timeline" && timelineItems ? (
          /* 整理任务=按时间线：跨工作区拍平的扁平任务列表 */
          <ul className="space-y-1 px-2 pt-1">
            {timelineItems.map(({ task, workspacePath: taskWorkspacePath, workspaceLabel: itemWorkspaceLabel }) => {
              const badge = getPersonaChatBadge(task);
              const title = badge
                ? stripPersonaTitlePrefix(task.title, badge.name)
                : task.title;
              return (
                <li key={`${taskWorkspacePath}:${task.taskId}`}>
                  <button
                    type="button"
                    onClick={() =>
                      openChat({
                        workspacePath: taskWorkspacePath,
                        sessionId: task.taskId,
                        title: task.title,
                      })
                    }
                    className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left active:bg-muted"
                  >
                    <span
                      className={cn(
                        "h-2 w-2 shrink-0 rounded-full",
                        task.status
                          ? (TASK_STATUS_DOT_CLASS[task.status] ?? "bg-muted-foreground/40")
                          : "bg-muted-foreground/40",
                      )}
                      aria-hidden
                    />
                    <span className="min-w-0 flex-1">
                      <span className="flex min-w-0 items-center gap-1.5">
                        <span className="min-w-0 truncate text-sm text-foreground">
                          {title || intl.formatMessage({ id: "taskList.untitled" })}
                        </span>
                        {badge ? (
                          <span
                            className={cn(
                              "flex shrink-0 items-center gap-1 rounded-[4px] px-1 leading-none",
                              SUBAGENT_COLOR_CLASS[
                                badge.color ?? resolveSubagentColorFromName(badge.name)
                              ],
                            )}
                          >
                            <Bot className="size-3 shrink-0" />
                            <span className="min-w-0 truncate text-[10px]">{badge.name}</span>
                          </span>
                        ) : null}
                      </span>
                      <span className="mt-0.5 block text-xs text-muted-foreground">
                        {itemWorkspaceLabel} ·{" "}
                        {intl.formatMessage(
                          { id: "webRemoteControl.mobileHome.updatedAt" },
                          { time: formatRelativeTime(task.updatedAt, Date.now()) },
                        )}
                      </span>
                    </span>
                    <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground/50" />
                  </button>
                </li>
              );
            })}
          </ul>
        ) : (
          <div className="space-y-2">
            {cards.map((card) => {
              const group = groups.find((item) => item.workspacePath === card.workspacePath);
              const key = card.workspacePath;
              const expanded = expandedKeys.has(key);
              const latestUpdatedAt = group?.items[0]?.updatedAt;
              return (
                <section
                  key={key}
                  className="overflow-hidden rounded-xl border border-border bg-card"
                >
                  <div className="flex items-center gap-3 px-3 py-3">
                    <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-muted">
                      {card.purpose === "conversation" ? (
                        <MessageSquarePlus className="h-4 w-4 text-muted-foreground" />
                      ) : (
                        <Folder className="h-4 w-4 text-muted-foreground" />
                      )}
                    </span>
                    <button
                      type="button"
                      onClick={() => toggleCard(key)}
                      className="min-w-0 flex-1 text-left"
                    >
                      <span className="flex items-center gap-1.5">
                        <span className="truncate text-sm font-medium text-foreground">
                          {card.label}
                        </span>
                        <span className="shrink-0 rounded-full bg-muted px-1.5 py-0.5 text-[10px] leading-none text-muted-foreground">
                          {card.purpose === "conversation"
                            ? intl.formatMessage({
                                id: "webRemoteControl.mobileHome.workspaceKind.conversation",
                              })
                            : intl.formatMessage({
                                id: "webRemoteControl.mobileHome.workspaceKind.local",
                              })}
                        </span>
                      </span>
                      <span className="mt-0.5 block truncate font-mono text-xs text-muted-foreground">
                        {card.workspacePath}
                      </span>
                      <span className="mt-0.5 block text-xs text-muted-foreground">
                        {latestUpdatedAt
                          ? intl.formatMessage(
                              { id: "webRemoteControl.mobileHome.updatedAt" },
                              {
                                time: formatRelativeTime(latestUpdatedAt, Date.now()),
                              },
                            )
                          : null}
                      </span>
                    </button>
                    <button
                      type="button"
                      onClick={() => toggleCard(key)}
                      className="flex shrink-0 items-center gap-0.5 text-xs text-muted-foreground"
                    >
                      {intl.formatMessage(
                        { id: "webRemoteControl.mobileHome.taskCount" },
                        { count: group?.total ?? 0 },
                      )}
                      <ChevronRight
                        className={cn("h-4 w-4 transition-transform", expanded && "rotate-90")}
                      />
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        openChat({
                          workspacePath: card.workspacePath,
                          sessionId: null,
                          title: intl.formatMessage({ id: "taskList.untitled" }),
                        })
                      }
                      className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-border text-muted-foreground active:bg-muted"
                      aria-label={intl.formatMessage({
                        id: "webRemoteControl.mobileHome.newTask",
                      })}
                    >
                      <Plus className="h-4 w-4" />
                    </button>
                  </div>
                  {expanded ? (
                    group && group.items.length > 0 ? (
                      <ul className="border-t border-border px-1 pb-1 pt-1">
                        {applyDerivedPersonaChatBadges(
                          group.items,
                          projectAgents.agentsByWorkspaceKey.get(group.workspacePath) ?? [],
                        ).map((task) => {
                          const personaChatBadge = getPersonaChatBadge(task);
                          const taskTitle = personaChatBadge
                            ? stripPersonaTitlePrefix(
                                task.title || intl.formatMessage({ id: "taskList.untitled" }),
                                personaChatBadge.name,
                              )
                            : task.title || intl.formatMessage({ id: "taskList.untitled" });
                          return (
                            <li key={task.taskId}>
                              <button
                                type="button"
                                onClick={() =>
                                  openChat({
                                    workspacePath: card.workspacePath,
                                    sessionId: task.taskId,
                                    title: task.title,
                                  })
                                }
                                className="flex w-full items-center gap-2.5 rounded-lg px-2 py-2.5 text-left active:bg-muted"
                              >
                                <span
                                  className={cn(
                                    "h-1.5 w-1.5 shrink-0 rounded-full",
                                    task.status
                                      ? (TASK_STATUS_DOT_CLASS[task.status] ??
                                        "bg-muted-foreground/40")
                                      : "bg-muted-foreground/40",
                                  )}
                                  aria-hidden
                                />
                                <span className="min-w-0 flex-1">
                                  <span className="flex min-w-0 items-center gap-1.5">
                                    <span className="min-w-0 truncate text-sm text-foreground">
                                      {taskTitle}
                                    </span>
                                    {personaChatBadge ? (
                                      <span
                                        className={cn(
                                          "flex shrink-0 items-center gap-1 rounded-[4px] px-1 leading-none",
                                          SUBAGENT_COLOR_CLASS[
                                            personaChatBadge.color ??
                                              resolveSubagentColorFromName(personaChatBadge.name)
                                          ],
                                        )}
                                      >
                                        <Bot className="size-3 shrink-0" />
                                        <span className="min-w-0 truncate text-[10px]">
                                          {personaChatBadge.name}
                                        </span>
                                      </span>
                                    ) : null}
                                  </span>
                                  <span className="mt-0.5 block text-xs text-muted-foreground">
                                    {intl.formatMessage(
                                      { id: "webRemoteControl.mobileHome.updatedAt" },
                                      { time: formatRelativeTime(task.updatedAt, Date.now()) },
                                    )}
                                  </span>
                                </span>
                                <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground/50" />
                              </button>
                            </li>
                          );
                        })}
                        {group.hasMore ? (
                          <li className="px-2 py-2 text-xs text-muted-foreground">
                            {intl.formatMessage({
                              id: "webRemoteControl.mobileHome.moreInDesktop",
                            })}
                          </li>
                        ) : null}
                      </ul>
                    ) : (
                      <p className="border-t border-border px-3 py-3 text-xs text-muted-foreground">
                        {intl.formatMessage({
                          id: "webRemoteControl.mobileHome.workspaceEmpty",
                        })}
                      </p>
                    )
                  ) : null}
                </section>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

