// ============================================================
// stall-sentinel 的可运行检查（node --test，Node 24 原生剥离类型直跑）
// ============================================================
// 不触真实 abort 路径：这里只有哨兵本身与假时钟（t.mock.timers 同时接管
// setTimeout 与 Date，idleMs 因此是精确值而不是近似值）。
//
// 运行：node --test apps/zcode-cli/packages/core/src/subagent/stall-sentinel.test.ts
// （core 的 tsconfig 已排除 **/*.test.ts，tsc 构建不受影响。）

import assert from "node:assert/strict";
import { test } from "node:test";
import { createStallSentinel, type StallObservation } from "./stall-sentinel.ts";

/** 收集停滞观察的哨兵替身。 */
function harness() {
  const observations: StallObservation[] = [];
  const sentinel = createStallSentinel({
    afterMs: 1_000,
    onStalled: (observation) => observations.push(observation),
  });
  return { observations, sentinel };
}

test("quiet segment reports exactly once with precise idleMs", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"] });
  const { observations, sentinel } = harness();

  sentinel.noteActivity();
  t.mock.timers.tick(1_000);
  assert.deepEqual(
    observations.map(({ idleMs, lastActivityAt }) => ({ idleMs, lastActivityAt })),
    [{ idleMs: 1_000, lastActivityAt: 0 }],
  );

  // 一段安静期至多一条：继续安静不重复刷屏。
  t.mock.timers.tick(10_000);
  assert.equal(observations.length, 1, "second fire in the same quiet segment must not happen");
});

test("activity resets the window and re-arms", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"] });
  const { observations, sentinel } = harness();

  sentinel.noteActivity();
  t.mock.timers.tick(600);
  sentinel.noteActivity();
  t.mock.timers.tick(600);
  assert.equal(observations.length, 0, "600ms after the last activity is not a stall yet");

  t.mock.timers.tick(400);
  assert.equal(observations.length, 1);
  assert.equal(observations[0]!.idleMs, 1_000, "idleMs counts from the LAST activity");
});

test("a new quiet segment may report again after activity", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"] });
  const { observations, sentinel } = harness();

  sentinel.noteActivity();
  t.mock.timers.tick(1_000);
  assert.equal(observations.length, 1);

  sentinel.noteActivity();
  t.mock.timers.tick(1_000);
  assert.equal(observations.length, 2, "next segment reports once more");
  assert.equal(observations[1]!.idleMs, 1_000);
});

test("stop cancels the pending report", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"] });
  const { observations, sentinel } = harness();

  sentinel.noteActivity();
  sentinel.stop();
  t.mock.timers.tick(10_000);
  assert.equal(observations.length, 0);
});

test("stop after a report keeps the segment settled", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"] });
  const { observations, sentinel } = harness();

  sentinel.noteActivity();
  t.mock.timers.tick(1_000);
  sentinel.stop();
  t.mock.timers.tick(10_000);
  assert.equal(observations.length, 1);
});

test("non-finite or non-positive afterMs degrades to a no-op", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"] });
  for (const afterMs of [0, -5, Number.NaN, Number.POSITIVE_INFINITY]) {
    const observations: StallObservation[] = [];
    const sentinel = createStallSentinel({ afterMs, onStalled: (o) => observations.push(o) });
    sentinel.noteActivity();
    t.mock.timers.tick(60_000);
    assert.equal(observations.length, 0, `afterMs=${afterMs} must never report`);
    sentinel.stop();
  }
});
