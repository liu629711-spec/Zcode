import { useEffect, useMemo, useRef, useState } from "react";
import type { IPlatformService, TaskNotificationPayload } from "@zcode/shared";
import type { ConversationSnapshot, SessionSummary } from "@zcode/shared/zcode-protocol-v4";
import { useServices } from "@/hooks/useServices.js";
import type { IntlInstance } from "@/i18n/index.js";
import { logger } from "@/logger.js";
import {
  collectPendingInteractionNotificationPayloads,
  collectSessionInteractionNotificationPayloads,
  collectTerminalTaskNotificationPayloads,
} from "@/lib/taskNotificationOrchestrator.js";
import {
  acquireSessionsIndex,
  releaseSessionsIndex,
  type SessionsIndexScope,
} from "@/v4/sessionsIndexRegistry.js";

type FormatMessage = IntlInstance["formatMessage"];
type TaskNotificationPlatform = Pick<IPlatformService, "showTaskNotification">;

interface PendingInteractionTaskNotificationsParams {
  snapshot: ConversationSnapshot | null;
  enabled: boolean;
  platform: TaskNotificationPlatform | null | undefined;
  formatMessage: FormatMessage;
}

function toSessionMap(sessions: readonly SessionSummary[]): Map<string, SessionSummary> {
  return new Map(sessions.map((session) => [session.sessionId, session]));
}

function showTaskNotification(
  platform: TaskNotificationPlatform,
  payload: TaskNotificationPayload,
): void {
  try {
    platform.showTaskNotification(payload);
  } catch (error) {
    logger.warn("[task-notification] 触发平台通知失败", {
      taskId: payload.taskId,
      status: payload.status,
      error,
    });
  }
}

/**
 * v4 任务通知编排（通知中枢 2026-10-12 升级为多工作区）。
 *
 * v4 重构删掉旧 renderer background monitor 后，platform 通知通道仍在，
 * 但 sessions-index 事实没有再被翻译成展示命令，导致任务完成/失败没有系统通知。
 * 这里只在 renderer 做“已观察边沿”的通知意图，事实仍以 sessions-index 为准；
 * 是否因窗口活跃而抑制通知继续交给 desktop/web platform 层判断。
 *
 * 覆盖面：调用方传入**全部本地 workspace** 的 scope——每个 scope 一条
 * sessions-index 订阅（registry 引用计数复用），两类边沿都发：
 * ①终态（completed/failed，跨 workspace；此前只有活跃 workspace 有）；
 * ②阻塞交互（员工提问/待批准，此前只有"正看着的那个会话"挂快照钩子才发——
 * 后台会话找老板整条静音，正是待批聚合要治的洞）。摘要版正文用会话标题，
 * 与快照版并存时 requestId 相同，主进程 3 秒去重窗兜住重复。
 */
export interface WorkspaceNotificationScope {
  workspacePath: string;
  workspaceIdentity?: string;
}

function scopeKeyOf(scope: WorkspaceNotificationScope): string {
  return [scope.workspaceIdentity ?? "", scope.workspacePath].join("\0");
}

interface ScopeIndexState {
  sessions: readonly SessionSummary[];
  live: boolean;
}

export function useWorkspaceTaskNotifications({
  scopes,
  enabled,
  platform,
  formatMessage,
}: {
  scopes: readonly WorkspaceNotificationScope[];
  enabled: boolean;
  platform: TaskNotificationPlatform | null | undefined;
  formatMessage: FormatMessage;
}): void {
  const { zcodeAgentService } = useServices();
  // scopes 数组逐渲染新建：订阅只认 scope 集合的签名，集合没变不重订阅。
  const scopesSignature = useMemo(
    () => scopes.map((scope) => scopeKeyOf(scope)).join("\u0001"),
    [scopes],
  );
  const scopesRef = useRef(scopes);
  scopesRef.current = scopes;
  const [byScope, setByScope] = useState<Record<string, ScopeIndexState>>({});
  // 终态边沿的"上一帧"按 scope 各记一份；每个 scope 首帧只作基线不发声。
  const previousByScopeRef = useRef<Map<string, Map<string, SessionSummary>>>(new Map());
  const baselineDoneRef = useRef<Set<string>>(new Set());
  // 已发过通知的 interactionId 全局一份；交互消失即摘除（同一处关掉再问会再响）。
  const seenInteractionIdsRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (!enabled || !platform || scopesRef.current.length === 0) {
      setByScope({});
      previousByScopeRef.current = new Map();
      baselineDoneRef.current = new Set();
      seenInteractionIdsRef.current = new Set();
      return;
    }
    // 开/关 tab：新 scope 首帧作基线；老 scope 的基线与 seen 保留。
    const acquired: Array<{
      scope: SessionsIndexScope;
      unsubscribe: () => void;
      store: ReturnType<typeof acquireSessionsIndex>;
    }> = [];
    const activeKeys = new Set(scopesRef.current.map(scopeKeyOf));
    for (const key of previousByScopeRef.current.keys()) {
      if (!activeKeys.has(key)) {
        previousByScopeRef.current.delete(key);
        baselineDoneRef.current.delete(key);
      }
    }
    for (const scope of scopesRef.current) {
      const key = scopeKeyOf(scope);
      const registryScope: SessionsIndexScope = {
        workspaceKey: scope.workspaceIdentity?.trim() || scope.workspacePath,
        workspacePath: scope.workspacePath,
        ...(scope.workspaceIdentity ? { workspaceIdentity: scope.workspaceIdentity } : {}),
      };
      const store = acquireSessionsIndex(registryScope, zcodeAgentService);
      const syncState = () => {
        setByScope((previous) => ({
          ...previous,
          [key]: { sessions: store.getSessions(), live: store.getStatus() === "live" },
        }));
      };
      syncState();
      const unsubscribe = store.subscribe(syncState);
      acquired.push({ scope: registryScope, unsubscribe, store });
    }
    return () => {
      for (const { scope, unsubscribe, store } of acquired) {
        unsubscribe();
        releaseSessionsIndex(scope, store);
      }
    };
  }, [enabled, platform, scopesSignature, zcodeAgentService]);

  useEffect(() => {
    if (!enabled || !platform) {
      return;
    }
    const previousByScope = previousByScopeRef.current;
    const baselineDone = baselineDoneRef.current;
    const seen = seenInteractionIdsRef.current;
    const payloads: TaskNotificationPayload[] = [];
    const pendingInteractionIds = new Set<string>();
    const liveSummaries: SessionSummary[] = [];
    for (const [key, state] of Object.entries(byScope)) {
      if (!state.live) continue;
      liveSummaries.push(...state.sessions);
      const nextMap = toSessionMap(state.sessions);
      const previousMap = previousByScope.get(key);
      if (!previousMap || !baselineDone.has(key)) {
        // 该 scope 的首帧：只立基线（重启/新开 tab 不重放旧 pending 与旧终态）。
        previousByScope.set(key, nextMap);
        baselineDone.add(key);
      } else {
        payloads.push(
          ...collectTerminalTaskNotificationPayloads({
            previousBySessionId: previousMap,
            sessions: state.sessions,
            formatMessage,
          }),
        );
        previousByScope.set(key, nextMap);
      }
    }
    for (const session of liveSummaries) {
      if (session.pendingInteraction) {
        pendingInteractionIds.add(session.pendingInteraction.interactionId);
      }
    }
    payloads.push(
      ...collectSessionInteractionNotificationPayloads({
        summaries: liveSummaries,
        seenInteractionIds: seen,
        formatMessage,
      }),
    );
    // seen 剪枝：还挂着的保留，消失的移除——交互被答掉后同一处再问会再响。
    const nextSeen = new Set<string>();
    for (const interactionId of seen) {
      if (pendingInteractionIds.has(interactionId)) {
        nextSeen.add(interactionId);
      }
    }
    for (const interactionId of pendingInteractionIds) {
      nextSeen.add(interactionId);
    }
    seenInteractionIdsRef.current = nextSeen;
    for (const payload of payloads) {
      showTaskNotification(platform, payload);
    }
  }, [byScope, enabled, formatMessage, platform]);
}

/**
 * 当前 conversation snapshot 的阻塞交互通知。
 *
 * 首个 snapshot 只作为基线，避免订阅历史/恢复 replayable snapshot 时重放旧权限弹窗通知；
 * 后续新增 interactionId 才发通知，重复 pending 不会刷屏。
 */
export function usePendingInteractionTaskNotifications({
  snapshot,
  enabled,
  platform,
  formatMessage,
}: PendingInteractionTaskNotificationsParams): void {
  const seenRef = useRef<{ sessionId: string; seenRequestIds: Set<string> } | null>(null);

  useEffect(() => {
    if (!enabled || !snapshot) {
      seenRef.current = null;
      return;
    }

    const currentRequestIds = snapshot.pendingInteractions.map(
      (interaction) => interaction.interactionId,
    );
    const currentSeen = seenRef.current;
    if (!currentSeen || currentSeen.sessionId !== snapshot.sessionId) {
      seenRef.current = {
        sessionId: snapshot.sessionId,
        seenRequestIds: new Set(currentRequestIds),
      };
      return;
    }

    if (platform) {
      const payloads = collectPendingInteractionNotificationPayloads({
        snapshot,
        seenRequestIds: currentSeen.seenRequestIds,
        formatMessage,
      });
      for (const payload of payloads) {
        showTaskNotification(platform, payload);
      }
    }

    for (const requestId of currentRequestIds) {
      currentSeen.seenRequestIds.add(requestId);
    }
  }, [enabled, formatMessage, platform, snapshot]);
}
