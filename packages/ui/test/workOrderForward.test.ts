// ============================================================
// 派单回执标题解析纯规则的可运行检查（tsx --test）：
// completed 标题解出交付方、失败/中断标题解出原员工（一键重派找人对号）。
// 「转交」的拼任务/选目标规则已随功能移除（2026-10-02）。
//
// 运行：npx tsx --test packages/ui/test/workOrderForward.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import { parseReceiptDelivererName } from "../src/v4/workOrderForward.js";

test("parseReceiptDelivererName：completed 标题解出交付方，失败/中断不返回名字", () => {
  assert.equal(parseReceiptDelivererName("code-builder 交活"), "code-builder");
  assert.equal(parseReceiptDelivererName("prd-engineer 的工单未完成"), undefined);
  assert.equal(parseReceiptDelivererName("frontend-design 的工单被中断"), undefined);
  assert.equal(parseReceiptDelivererName("交活"), undefined);
  assert.equal(parseReceiptDelivererName("随便一句话"), undefined);
});
