// ============================================================
// workflow run 回放的**取数**：逐页拉 journal 事件 + 封顶（不碰 React 的纯异步函数）
// ============================================================
// 从 useWorkflowRunReplay 抽出来的分页循环：分页与封顶是纯数据操作，React 只负责门控与
// 迟到丢弃，抽开后 node:test 用假分页器就能钉住截断语义（恰满 cap ≠ 截断）。
//
// cursor 语义是「严格大于」，journal 条目不可变（sequence 契约），读过程中新落的事件只会
// 出现在下一页的尾部——不重不漏，无需快照锁。上界 {@link REPLAY_MAX_EVENTS} 只防「一个失控
// run 把渲染器内存吃穿」：到界即停，`truncated` 置真——回放少一截尾巴，比渲染器先死，是
// 正确的取舍。折叠本身是一次 O(n) 单遍扫，几千事件毫秒级，不在意；在意的是 n 失控，所以
// 界卡在取数这层。

import type { WorkflowRunReplayEvent } from "@zcode/shared/zcode-protocol-v4";

/** 事件数上界：防失控 run 吃穿渲染器内存。到界且末页还有更多 = 截断。 */
export const REPLAY_MAX_EVENTS = 20_000;
/** 每页条数；与 CLI 侧允许的单页上界（500）一致，少几次往返。 */
export const REPLAY_PAGE_SIZE = 500;

/** `workflowRunEvents` 查询的一页（与 v4 结果同形；测试用假分页器照此仿造）。 */
export interface ReplayEventPage {
  events: WorkflowRunReplayEvent[];
  hasMore: boolean;
}

/** 一次取数的产出：全量事件 + 是否被上界截断。 */
export interface ReplayEventCollection {
  events: WorkflowRunReplayEvent[];
  /** 事件数撞上 {@link REPLAY_MAX_EVENTS} 且末页还有更多——回放少了尾巴。 */
  truncated: boolean;
}

/**
 * 逐页累进取全量事件，到没有更多或撞上封顶为止。
 *
 * 截断的判据是**末页的 hasMore**：恰满上界且没有更多 = 读尽了，不是截断；到界了但末页说
 * 后面还有，才算。`shouldContinue` 在每页到手后问一次，返回 false 即丢弃当页、不再续翻——
 * 调用方切 run / 卸载时省掉白跑的分页（迟到的页本来也不许污染新证据）。
 */
export async function collectReplayEvents(options: {
  fetchPage: (afterSequence: number | undefined) => Promise<ReplayEventPage>;
  shouldContinue?: () => boolean;
}): Promise<ReplayEventCollection> {
  const { fetchPage, shouldContinue } = options;
  const collected: WorkflowRunReplayEvent[] = [];
  let afterSequence: number | undefined;
  let truncated = false;
  while (collected.length < REPLAY_MAX_EVENTS) {
    const page = await fetchPage(afterSequence);
    if (shouldContinue?.() === false) break;
    collected.push(...page.events);
    if (!page.hasMore) break;
    // 恰满上界且末页说没有更多 = 读尽了；截断只认「到界了但还有更多」。
    if (collected.length >= REPLAY_MAX_EVENTS) {
      truncated = true;
      break;
    }
    const last = page.events.at(-1)?.sequence;
    if (last === undefined) break;
    afterSequence = last;
  }
  return { events: collected, truncated };
}
