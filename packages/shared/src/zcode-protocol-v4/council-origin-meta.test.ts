// ============================================================
// 圆桌会 originMeta（backgroundResultOriginMetaSchema.council*）的可运行检查
// ============================================================
// 钉子：zod 剥离未知键——contracts 的 BackgroundResultOriginMeta.council* 与
// shared 这份 schema 漂移时（漏加字段/闭集不一致），冷恢复后圆桌卡静默散架。
// 本检查保证席位回执轮头的圆桌会字段过 schema 后**原样存活**，且非圆桌轮
// （无 council* 字段）的解析口径不变。
//
// 本文件参与 tsc -b（shared 的 tsconfig 未排除 *.test.ts），import 一律 .js 后缀。
//
// 运行：npx tsx --test packages/shared/src/zcode-protocol-v4/council-origin-meta.test.ts

import assert from "node:assert/strict";
import { test } from "node:test";
import { backgroundResultOriginMetaSchema } from "./workflow-row-meta.js";

const SEAT_RECEIPT_ORIGIN_META = {
  backgroundSource: "agent_work_order_receipt",
  workId: "wo-1",
  title: "小验 交活",
  batchId: "batch-1",
  batchTitle: "登录页改造",
  councilId: "council-9",
  councilKind: "plan",
  councilRound: 2,
  councilPhase: "deliberation",
  councilSeat: { index: 3, lens: "edge" },
} as const;

test("圆桌会字段过 schema 原样存活（zod 剥未知键的钉子）", () => {
  const parsed = backgroundResultOriginMetaSchema.parse(SEAT_RECEIPT_ORIGIN_META);
  assert.equal(parsed.councilId, "council-9");
  assert.equal(parsed.councilKind, "plan");
  assert.equal(parsed.councilRound, 2);
  assert.equal(parsed.councilPhase, "deliberation");
  assert.deepEqual(parsed.councilSeat, { index: 3, lens: "edge" });
  // 既有字段不受圆桌会扩展影响（批次与圆桌同轮共存时都在）。
  assert.equal(parsed.batchId, "batch-1");
  assert.equal(parsed.batchTitle, "登录页改造");
});

test("非圆桌轮（无 council* 字段）解析口径不变", () => {
  const parsed = backgroundResultOriginMetaSchema.parse({
    backgroundSource: "agent_work_order_batch_qc",
    workId: "batch-1",
    title: "质检 · 登录页改造",
    batchId: "batch-1",
  });
  assert.equal(parsed.councilId, undefined);
  assert.equal(parsed.councilKind, undefined);
  assert.equal(parsed.councilRound, undefined);
  assert.equal(parsed.councilPhase, undefined);
  assert.equal(parsed.councilSeat, undefined);
});

test("圆桌会闭集执法：越界攻角/轮次/阶段解析失败，不静默换值", () => {
  assert.equal(
    backgroundResultOriginMetaSchema.safeParse({
      ...SEAT_RECEIPT_ORIGIN_META,
      councilSeat: { index: 0, lens: "vibes" },
    }).success,
    false,
  );
  assert.equal(
    backgroundResultOriginMetaSchema.safeParse({
      ...SEAT_RECEIPT_ORIGIN_META,
      councilRound: 3,
    }).success,
    false,
  );
  assert.equal(
    backgroundResultOriginMetaSchema.safeParse({
      ...SEAT_RECEIPT_ORIGIN_META,
      councilPhase: "pending",
    }).success,
    false,
  );
});
