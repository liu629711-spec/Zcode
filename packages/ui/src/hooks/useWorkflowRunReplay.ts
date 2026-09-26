import { useEffect, useRef, useState } from "react";
import {
  foldWorkflowRunReplay,
  type WorkflowRunReplayTimeline,
} from "@zcode/shared/zcode-protocol-v4";
import { logger } from "@/logger.js";
import { useV4Conversation } from "@/v4/V4ConversationContext.js";

/**
 * T1 行车记录仪：一个 workflow run 的**回放**数据。
 *
 * 数据源是既有的 `workflowRunEvents` 查询（journal `dwf_event` 的 sequence 分页），折叠用
 * @zcode/shared 的 `foldWorkflowRunReplay`——与 adapters 的服务端投影 `loadDwfRunReplay`
 * **同一份折叠核**：两边对同一份 journal 必须给出同一份证据，折叠不允许长两份。
 *
 * 分页循环到没有更多为止（cursor = journal sequence，永不失效，见 transport.ts 那边的论证）。
 * 上界 {@link REPLAY_MAX_EVENTS} 只防「一个失控 run 把渲染器内存吃穿」：到界即停，`truncated`
 * 置真——回放少一截尾巴，比渲染器先死，是正确的取舍。折叠本身是一次 O(n) 单遍扫，几千事件
 * 毫秒级，不在意；在意的是 n 失控，所以界卡在取数这层。
 */
const REPLAY_MAX_EVENTS = 20_000;
/** 每页条数；与 CLI 侧允许的单页上界（500）一致，少几次往返。 */
const REPLAY_PAGE_SIZE = 500;

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
  /** 事件数撞上 {@link REPLAY_MAX_EVENTS} 被截断。 */
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
    const collected: Array<{ sequence: number; type: string; payload: Record<string, unknown> }> = [];
    let afterSequence: number | undefined;
    let alive = true;
    void (async () => {
      try {
        let truncated = false;
        // 逐页累进：cursor 语义是「严格大于」，journal 条目不可变（sequence 契约），
        // 读过程中新落的事件只会出现在下一页的尾部——不重不漏，无需快照锁。
        while (collected.length < REPLAY_MAX_EVENTS) {
          const page = await workflowRunEvents({
            sessionId,
            runId,
            ...(afterSequence === undefined ? {} : { afterSequence }),
            limit: REPLAY_PAGE_SIZE,
          });
          if (requestVersion !== requestVersionRef.current || !alive) return;
          collected.push(...page.events);
          if (!page.hasMore) break;
          const last = page.events.at(-1)?.sequence;
          if (last === undefined) break;
          afterSequence = last;
        }
        if (collected.length >= REPLAY_MAX_EVENTS) truncated = true;
        const timeline = foldWorkflowRunReplay(collected);
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
