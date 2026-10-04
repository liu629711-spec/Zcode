// ============================================================
// 归档补时间戳（地基清理 2026-10-04）的可运行检查。
// 驱动**真实** server-operations.archiveSession op（不是镜像副本）：
// 会话库假 store 只记录 updateSession 入参。验证：
//  1. archived=true → timeArchived 写入当前毫秒时间戳；
//  2. archived=false → timeArchived 明确置 null（updateSession 口径：
//     undefined=保留旧值，null=清除，必须传 null）；
//  3. 会话行不存在（updateSession 抛 Session not found）→ 返回 archived:false，
//     op 不炸——task 索引里可能有没落过会话库的历史行；
//  4. 没配 sessionStore → 返回 archived:false。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/session-archive.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { archiveSession } from "../src/zcode-protocol/server-operations.js";

interface UpdateCall {
  id: string;
  timeArchived?: number | null;
}

function makeContext(behavior?: (input: UpdateCall) => Promise<unknown>) {
  const calls: UpdateCall[] = [];
  const store = {
    async updateSession(input: UpdateCall) {
      calls.push(input);
      if (behavior) return behavior(input);
      return {};
    },
  };
  const context = { deps: { sessionStore: store }, logger: undefined };
  return { context: context as never, calls };
}

test("归档：timeArchived 写当前毫秒时间戳", async () => {
  const { context, calls } = makeContext();
  const before = Date.now();
  const result = await archiveSession(context, { sessionId: "sess-1", archived: true });
  assert.deepEqual(result, { archived: true });
  assert.equal(calls.length, 1);
  assert.equal(calls[0]!.id, "sess-1");
  const stamp = calls[0]!.timeArchived;
  assert.equal(typeof stamp, "number");
  assert.ok(stamp! >= before && stamp! <= Date.now(), "时间戳应是当前时刻");
});

test("取消归档：timeArchived 置 null（undefined 会保留旧值）", async () => {
  const { context, calls } = makeContext();
  const result = await archiveSession(context, { sessionId: "sess-1", archived: false });
  assert.deepEqual(result, { archived: true });
  assert.equal(calls.length, 1);
  assert.equal(calls[0]!.timeArchived, null);
});

test("会话行不存在：返回 archived:false，op 不炸", async () => {
  const { context, calls } = makeContext(async () => {
    throw new Error("Session not found: sess-gone");
  });
  const result = await archiveSession(context, { sessionId: "sess-gone", archived: true });
  assert.deepEqual(result, { archived: false });
  assert.equal(calls.length, 1);
});

test("没配 sessionStore：返回 archived:false", async () => {
  const context = { deps: {}, logger: undefined } as never;
  const result = await archiveSession(context, { sessionId: "sess-1", archived: true });
  assert.deepEqual(result, { archived: false });
});
