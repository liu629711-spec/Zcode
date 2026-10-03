// 圆桌会证据 hook（2026-10-03 独立页刀5）：council/evidence 只读查询 +
// selectCouncilMeetings 派生模型 + councilStageStore 同步。
//
// 数据洞补口：会议模型只能从召集方会话的轮证据聚合，而 ConversationTimeline
// 只在会话打开时同步 store——独立页（跨会话看会）靠本 hook 直接向协议端取
// 证据。派生出的模型写回 councilStageStore（syncCouncilMeetings），此后会话内
// 画布/聚焦握手同样吃得到；timeline 挂载后会用活投影覆盖，互不冲突。
//
// 刷新口径：台账（council/list 轮询）里该会议的 timeUpdated 变化才重取证据
// ——账没动就不打 RPC；会议推进（回执落账/主席轮到账）必然推动 timeUpdated。

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CouncilMeetingModel } from "@/v4/councilMeeting.js";
import { selectCouncilMeetings } from "@/v4/councilMeeting.js";
import { logger } from "@/logger.js";
import { useBaseWorkspaceServices } from "@/hooks/useWorkspaceServices.js";
import { useRemoteWorkspaceSessionStore } from "@/store/remoteWorkspaceSessionStore.js";
import { resolveWorkspaceServices } from "@/lib/workspaceServiceResolver.js";
import { useCouncilStageStore } from "@/store/councilStageStore.js";

export interface CouncilEvidenceState {
  /** 选中会议的模型（证据未到/协议端未升级 = null）。 */
  meeting: CouncilMeetingModel | null;
  /** 首次查询未落定。 */
  loading: boolean;
  /** 查询失败（断连/旧协议端）；重试走 refresh。 */
  error: boolean;
  refresh: () => void;
}

export function useCouncilEvidence(params: {
  workspacePath: string;
  workspaceIdentity?: string;
  /** 召集方会话（台账行带）；null = 页面空态，不查询。 */
  sessionId: string | null;
  councilId: string | null;
  /** 台账侧该会议的最近落账时间：变化即证据可能增长，触发重取。 */
  timeUpdated?: number;
}): CouncilEvidenceState {
  const baseServices = useBaseWorkspaceServices();
  const sessionsById = useRemoteWorkspaceSessionStore((state) => state.sessionsById);
  const sessionIdByWorkspaceIdentity = useRemoteWorkspaceSessionStore(
    (state) => state.sessionIdByWorkspaceIdentity,
  );
  const sessionIdByWorkspacePath = useRemoteWorkspaceSessionStore(
    (state) => state.sessionIdByWorkspacePath,
  );
  const syncCouncilMeetings = useCouncilStageStore((state) => state.syncCouncilMeetings);
  const [meeting, setMeeting] = useState<CouncilMeetingModel | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  const [refreshTick, setRefreshTick] = useState(0);
  const requestIdRef = useRef(0);
  const refresh = useCallback(() => setRefreshTick((tick) => tick + 1), []);

  const { sessionId, councilId } = params;
  useEffect(() => {
    if (!sessionId || !councilId) {
      setMeeting(null);
      setError(false);
      setLoading(false);
      return;
    }
    const resolved = resolveWorkspaceServices(
      {
        workspacePath: params.workspacePath,
        ...(params.workspaceIdentity ? { workspaceIdentity: params.workspaceIdentity } : {}),
      },
      baseServices,
      { sessionsById, sessionIdByWorkspaceIdentity, sessionIdByWorkspacePath },
    );
    if (!resolved) return;
    const requestId = ++requestIdRef.current;
    setLoading(true);
    resolved.services.zcodeAgentService
      .getCouncilEvidence({
        workspacePath: params.workspacePath,
        ...(params.workspaceIdentity ? { workspaceIdentity: params.workspaceIdentity } : {}),
        sessionId,
        councilId,
      })
      .then((result) => {
        if (requestIdRef.current !== requestId) return;
        const models = selectCouncilMeetings(result.units);
        const next = models.find((candidate) => candidate.councilId === councilId) ?? null;
        setMeeting(next);
        setError(false);
        // 空证据不写 store：协议端未升级时别把会话内 timeline 可能已同步的
        // 活模型抹掉（写 store 是补洞，不是清场）。
        if (models.length > 0) syncCouncilMeetings(sessionId, models);
      })
      .catch((cause: unknown) => {
        if (requestIdRef.current !== requestId) return;
        setMeeting(null);
        setError(true);
        logger.warn("[useCouncilEvidence] 会议证据查询失败", cause);
      })
      .finally(() => {
        if (requestIdRef.current !== requestId) return;
        setLoading(false);
      });
    // serviceResolverState 按引用每次重建（同 useWorkspaceCouncilMeetings 纪律），
    // 只认 workspace、目标与 base services；timeUpdated 变化（台账轮询推进）即重取。
  }, [
    baseServices,
    councilId,
    params.timeUpdated,
    params.workspaceIdentity,
    params.workspacePath,
    refreshTick,
    sessionId,
    sessionsById,
    syncCouncilMeetings,
  ]);

  return useMemo(
    () => ({ meeting, loading, error, refresh }),
    [meeting, loading, error, refresh],
  );
}
