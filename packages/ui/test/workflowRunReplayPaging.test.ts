import assert from "node:assert/strict";
import test from "node:test";
import {
  collectReplayEvents,
  REPLAY_MAX_EVENTS,
  type ReplayEventPage,
} from "../src/hooks/workflowRunReplayPaging.js";

// ============================================================
// 回放取数分页循环的可运行检查（纯异步函数，不碰 React，假分页器直跑）
// ============================================================
// 钉的是截断判据：截断 =「到界了但末页还有更多」，恰满 cap 且读尽 ≠ 截断。
// 运行：npx tsx --test packages/ui/test/workflowRunReplayPaging.test.ts

/** 假分页器：totalEvents 条 sequence 连续的事件、pageSize 每页，hasMore 按「后面还有」判定。 */
function fakePager(totalEvents: number, pageSize: number) {
  let calls = 0;
  return {
    calls: () => calls,
    fetchPage: async (afterSequence: number | undefined): Promise<ReplayEventPage> => {
      calls += 1;
      const start = afterSequence === undefined ? 0 : afterSequence + 1;
      const events: ReplayEventPage["events"] = [];
      for (
        let sequence = start;
        sequence < Math.min(totalEvents, start + pageSize);
        sequence += 1
      ) {
        events.push({ sequence, type: "node-queued", payload: {} });
      }
      return { events, hasMore: start + pageSize < totalEvents };
    },
  };
}

test("远小于 cap：全量收齐，不截断", async () => {
  const pager = fakePager(1_234, 500);
  const { events, truncated } = await collectReplayEvents({ fetchPage: pager.fetchPage });
  assert.equal(truncated, false);
  assert.equal(events.length, 1_234);
  assert.equal(events[0]?.sequence, 0);
  assert.equal(events.at(-1)?.sequence, 1_233);
  assert.equal(pager.calls(), 3);
});

test("恰满 cap 且末页没有更多：读尽了，不截断", async () => {
  const pager = fakePager(REPLAY_MAX_EVENTS, 500);
  const { events, truncated } = await collectReplayEvents({ fetchPage: pager.fetchPage });
  assert.equal(truncated, false);
  assert.equal(events.length, REPLAY_MAX_EVENTS);
  assert.equal(pager.calls(), REPLAY_MAX_EVENTS / 500);
});

test("到界了但末页还有更多：截断，且不再翻页", async () => {
  const pager = fakePager(REPLAY_MAX_EVENTS + 1_500, 500);
  const { events, truncated } = await collectReplayEvents({ fetchPage: pager.fetchPage });
  assert.equal(truncated, true);
  assert.equal(events.length, REPLAY_MAX_EVENTS);
  assert.equal(pager.calls(), REPLAY_MAX_EVENTS / 500);
});

test("hasMore 谎报但事件为空：停下，不截断", async () => {
  const page: ReplayEventPage = { events: [], hasMore: true };
  const { events, truncated } = await collectReplayEvents({ fetchPage: async () => page });
  assert.equal(truncated, false);
  assert.equal(events.length, 0);
});

test("过期请求：当页即弃，不再续翻", async () => {
  const pager = fakePager(5_000, 500);
  const { events, truncated } = await collectReplayEvents({
    fetchPage: pager.fetchPage,
    // 第一页到手时仍活着；第二页到手时调用方已经切走（返回 false）。
    shouldContinue: () => pager.calls() < 2,
  });
  assert.equal(truncated, false);
  assert.equal(events.length, 500, "只保留第一页");
  assert.equal(pager.calls(), 2, "第二页已请求但被丢弃，第三页不再请求");
});
