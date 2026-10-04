// ============================================================
// 删任务清聊天内容（地基清理 2026-10-04）op 层可运行检查：驱动**真实**
// server-operations.purgeSessionContent，会话库假 store 只记录/篡改行为。验证：
//  1. store 在场且删除成功 → purged:true；
//  2. 会话不存在（purgeSession 返回 false）→ purged:false，不报错；
//  3. store 删除抛错 → purged:false 且不上抛（tombstone 已落地，UI 不会再显示，
//     清理只是回收空间，不能让删除动作回滚或炸协议）；
//  4. store 缺席（旧实现）或 purgeSession 未实现 → purged:false。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/session-purge.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { purgeSessionContent } from "../src/zcode-protocol/server-operations.js";

function makeContext(purge?: (input: { id: string }) => Promise<boolean>) {
  const store = purge
    ? { purgeSession: purge }
    : {};
  return { deps: { sessionStore: store }, logger: undefined } as never;
}

test("删除成功：purged:true", async () => {
  let received: string | undefined;
  const result = await purgeSessionContent(makeContext(async (input) => {
    received = input.id;
    return true;
  }), { sessionId: "sess-1" });
  assert.deepEqual(result, { purged: true });
  assert.equal(received, "sess-1");
});

test("会话不存在：purged:false", async () => {
  const result = await purgeSessionContent(makeContext(async () => false), {
    sessionId: "sess-gone",
  });
  assert.deepEqual(result, { purged: false });
});

test("store 抛错：purged:false，op 不炸", async () => {
  const result = await purgeSessionContent(makeContext(async () => {
    throw new Error("db busy");
  }), { sessionId: "sess-1" });
  assert.deepEqual(result, { purged: false });
});

test("store 缺席 / purgeSession 未实现：purged:false", async () => {
  assert.deepEqual(await purgeSessionContent({ deps: {} } as never, { sessionId: "s" }), {
    purged: false,
  });
  assert.deepEqual(await purgeSessionContent(makeContext(undefined), { sessionId: "s" }), {
    purged: false,
  });
});
