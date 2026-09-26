import { useEffect, useRef, useState } from "react";
import {
  foldWorkflowRunReplay,
  type WorkflowRunReplayTimeline,
} from "@zcode/shared/zcode-protocol-v4";
import { logger } from "@/logger.js";
import { collectReplayEvents, REPLAY_PAGE_SIZE } from "@/hooks/workflowRunReplayPaging.js";
import { useV4Conversation } from "@/v4/V4ConversationContext.js";

/**
 * T1 行车记录仪：一个 workflow run 的**回放**数据。
 *
 * 数据源是既有的 `workflowRunEvents` 查询（journal `dwf_event` 的 sequence 分页），折叠用
 * @zcode/shared 的 `foldWorkflowRunReplay`——与 adapters 的服务端投影 `loadDwfRunReplay`
 * **同一份折叠核**：两边对同一份 journal 必须给出同一份证据，折叠不允许长两份。
 *
 * 取数循环（逐页 + 封顶 + 截断判据）抽在 workflowRunReplayPaging.ts：纯异步函数，不碰
 * React，假分页器单测钉住「恰满 cap ≠ 截断」——截断以末页 hasMore 为准。
 */

/**
 * 能力缺席（旧 CLI 没有这个 query / run service 没构造）与普通失败分开——与
 * useWorkflowRunJournalSummaries 同一条读法：错误跨 JSON-RPC 之后只剩 message 可靠，
 * reasonCode 与能力名两个模式都收。回放区对能力缺席整区缺席，对普通失败给一句可读的错误。
 */
function isReplayCapabilityMissing(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return message.includes("capabilityUnsupported") || message.includes("workflowRunEvents");
}

export type WorkflowRunReplayStatus =
  | "idle"
  | "loading"
  | "ready"
  | "empty"
  | "error"
  | "unavailable";

export interface WorkflowRunReplayState {
  status: WorkflowRunReplayStatus;
  timeline: WorkflowRunReplayTimeline | undefined;
  /** 事件数撞上取数上界（workflowRunReplayPaging 的 REPLAY_MAX_EVENTS）被截断。 */
  truncated: boolean;
  error: string | null;
}

/**
 * 抓全量 journal 事件并折叠成回放时间线。**取数由 `enabled` 门控**（回放区展开才取）：
 * 回放是审计动作，面板每次打开都为一份没人看的证据付一遍 journal 读，不划算；产物区
 * 常开常取是它的交付物属性，回放区不陪跑。重开（expanded false→true）/ 切 run 即重取。
 */
export function useWorkflowRunReplay(options: {
  sessionId: string;
  runId: string;
  /** false 时不取数，停在 idle（切 run 会清掉上一份证据）。 */
  enabled: boolean;
}): WorkflowRunReplayState {
  const { workflowRunEvents } = useV4Conversation();
  const { sessionId, runId } = options;
  const enabled =
    options.enabled && sessionId.length > 0 && runId.length > 0;
  const [state, setState] = useState<WorkflowRunReplayState>({
    status: "idle",
    timeline: undefined,
    truncated: false,
    error: null,
  });
  // 迟到响应必须被丢弃（切 run / 卸载后），不能污染新 run 的证据——与产物 hook 同一把闸。
  const requestVersionRef = useRef(0);

  useEffect(() => {
    const requestVersion = ++requestVersionRef.current;
    if (!enabled) {
      setState({ status: "idle", timeline: undefined, truncated: false, error: null });
      return;
    }
    setState({ status: "loading", timeline: undefined, truncated: false, error: null });
    let alive = true;
    void (async () => {
      try {
        // 迟到响应两道闸：shouldContinue 在每页到手后弃掉过期请求（省白跑的分页），
        // 归并完再查一次——折叠期间切换的请求不许污染新 run 的证据（与产物 hook 同一把闸）。
        const { events, truncated } = await collectReplayEvents({
          fetchPage: async (afterSequence) => {
            const page = await workflowRunEvents({
              sessionId,
              runId,
              ...(afterSequence === undefined ? {} : { afterSequence }),
              limit: REPLAY_PAGE_SIZE,
            });
            return { events: page.events, hasMore: page.hasMore };
          },
          shouldContinue: () => requestVersion === requestVersionRef.current && alive,
        });
        if (requestVersion !== requestVersionRef.current || !alive) return;
        const timeline = foldWorkflowRunReplay(events);
        setState({
          status: timeline.eventCount === 0 ? "empty" : "ready",
          timeline,
          truncated,
          error: null,
        });
      } catch (caught) {
        if (requestVersion !== requestVersionRef.current || !alive) return;
        if (isReplayCapabilityMissing(caught)) {
          setState({ status: "unavailable", timeline: undefined, truncated: false, error: null });
          return;
        }
        const message = caught instanceof Error ? caught.message : String(caught);
        logger.warn("[workflow-run] 回放读取失败", { error: message, runId, sessionId });
        setState({ status: "error", timeline: undefined, truncated: false, error: message });
      }
    })();
    return () => {
      alive = false;
    };
    // 折叠在 effect 里同步做：几千事件的 O(n) 单遍不值得为它加一层并发闸。
  }, [enabled, runId, sessionId, workflowRunEvents]);

  return state;
}
