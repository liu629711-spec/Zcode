// 会议室侧栏分组的数据 hook（2026-10-03 圆桌会刀1）：council/list 只读查询 +
// 低频轮询。会议推进没有专用推送事件（席位回执静默落账、主席轮随会话活动走），
// 侧栏目录用 15s 轮询兜住进行中→已收口的翻转。
// ponytail: 轮询是刻意的最小实现（一次小 RPC，读本地 sqlite）；协议端将来给
// 台账变更加推送事件时，把 interval 换成订阅即可，消费面（本 hook 返回形状）不变。

import { useCallback, useEffect, useRef, useState } from "react";
import type { ZCodeCouncilMeetingSummary } from "@zcode/shared";
import { logger } from "@/logger.js";
import { useBaseWorkspaceServices } from "@/hooks/useWorkspaceServices.js";
import { useRemoteWorkspaceSessionStore } from "@/store/remoteWorkspaceSessionStore.js";
import { resolveWorkspaceServices } from "@/lib/workspaceServiceResolver.js";

const COUNCIL_LIST_POLL_INTERVAL_MS = 15_000;

export interface WorkspaceCouncilMeetings {
  meetings: ZCodeCouncilMeetingSummary[];
  /** 首次查询未落定（三态之一；后续轮询静默刷新不置位）。 */
  loading: boolean;
  /** 查询失败（协议端未升级/断连）；重试按钮走 refresh。 */
  error: boolean;
  refresh: () => void;
}

export function useWorkspaceCouncilMeetings(params: {
  workspacePath: string;
  workspaceIdentity?: string;
}): WorkspaceCouncilMeetings {
  const baseServices = useBaseWorkspaceServices();
  const sessionsById = useRemoteWorkspaceSessionStore((state) => state.sessionsById);
  const sessionIdByWorkspaceIdentity = useRemoteWorkspaceSessionStore(
    (state) => state.sessionIdByWorkspaceIdentity,
  );
  const sessionIdByWorkspacePath = useRemoteWorkspaceSessionStore(
    (state) => state.sessionIdByWorkspacePath,
  );
  const serviceResolverState = {
    sessionsById,
    sessionIdByWorkspaceIdentity,
    sessionIdByWorkspacePath,
  };
  const [meetings, setMeetings] = useState<ZCodeCouncilMeetingSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [refreshTick, setRefreshTick] = useState(0);
  const requestIdRef = useRef(0);
  const hasResultRef = useRef(false);
  const refresh = useCallback(() => setRefreshTick((tick) => tick + 1), []);

  useEffect(() => {
    const resolved = resolveWorkspaceServices(
      {
        workspacePath: params.workspacePath,
        ...(params.workspaceIdentity ? { workspaceIdentity: params.workspaceIdentity } : {}),
      },
      baseServices,
      serviceResolverState,
    );
    if (!resolved) {
      // 远端目标还没解析到 session（断连占位）：保持现状，不误报空列表。
      return;
    }
    const requestId = ++requestIdRef.current;
    if (!hasResultRef.current) setLoading(true);
    resolved.services.zcodeAgentService
      .listCouncilMeetings({
        workspacePath: params.workspacePath,
        ...(params.workspaceIdentity ? { workspaceIdentity: params.workspaceIdentity } : {}),
      })
      .then((result) => {
        if (requestIdRef.current !== requestId) return;
        hasResultRef.current = true;
        setMeetings(result.meetings);
        setError(false);
      })
      .catch((cause: unknown) => {
        if (requestIdRef.current !== requestId) return;
        hasResultRef.current = true;
        setError(true);
        logger.warn("[useWorkspaceCouncilMeetings] 会议目录查询失败", cause);
      })
      .finally(() => {
        if (requestIdRef.current !== requestId) return;
        setLoading(false);
      });
    // serviceResolverState 按引用每次重建——它的字段都来自独立 store 选择器，
    // 依赖它会把 effect 打成每次渲染都跑；这里只认 workspace 与 base services。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    baseServices,
    params.workspaceIdentity,
    params.workspacePath,
    refreshTick,
    sessionsById,
  ]);

  useEffect(() => {
    const timer = window.setInterval(refresh, COUNCIL_LIST_POLL_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [refresh]);

  return { meetings, loading, error, refresh };
}
