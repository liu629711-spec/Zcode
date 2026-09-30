import assert from "node:assert/strict";
import { test } from "node:test";
import {
  DEFAULT_CONTEXT_HANDOVER_THRESHOLD_PERCENT,
  buildSessionContextBannerState,
  resolveContextHandoverThresholdPercent,
  resolveContextUsagePercent,
} from "../src/v4/sessionContextBannerState.js";

test("banner shows at/above threshold and stays hidden below (default 40)", () => {
  // 200k 窗口：78k = 39% 不提醒；80k = 40% → 提醒。
  const below = buildSessionContextBannerState({
    usedTokens: 78_000,
    maxTokens: 200_000,
    thresholdPercent: undefined,
  });
  assert.equal(below.visible, false);
  const at = buildSessionContextBannerState({
    usedTokens: 80_000,
    maxTokens: 200_000,
    thresholdPercent: undefined,
  });
  assert.equal(at.visible, true);
  assert.equal(at.percent, 40);
  assert.equal(
    resolveContextHandoverThresholdPercent(undefined),
    DEFAULT_CONTEXT_HANDOVER_THRESHOLD_PERCENT,
  );
});

test("threshold 0 disables the banner entirely", () => {
  const off = buildSessionContextBannerState({
    usedTokens: 190_000,
    maxTokens: 200_000,
    thresholdPercent: 0,
  });
  assert.equal(off.visible, false);
  assert.equal(resolveContextHandoverThresholdPercent(0), 0);
});

test("transient used=0 usage updates never trigger the banner", () => {
  const state = buildSessionContextBannerState({
    usedTokens: 0,
    maxTokens: 200_000,
    thresholdPercent: 40,
  });
  assert.equal(state.visible, false);
  assert.equal(state.percent, null);
  assert.equal(resolveContextUsagePercent(0, 200_000), null);
  assert.equal(resolveContextUsagePercent(100, 0), null);
});

test("percent clamps to 1..100", () => {
  assert.equal(resolveContextUsagePercent(150_000, 100_000), 100);
  assert.equal(resolveContextUsagePercent(1, 200_000), 1);
});
