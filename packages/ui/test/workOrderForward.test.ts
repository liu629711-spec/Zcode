// ============================================================
// 回执卡「转交」纯规则的可运行检查（tsx --test）：
// 交付方名字从 CLI 铸造的标题解出、转交目标剔除交付方本人、
// 成果原文全文进新工单（目标会话看不见发起方的回执）。
//
// 运行：npx tsx --test packages/ui/test/workOrderForward.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  buildWorkOrderForwardTask,
  parseReceiptDelivererName,
  selectForwardTargets,
} from "../src/v4/workOrderForward.js";

test("parseReceiptDelivererName：completed 标题解出交付方，失败/中断不返回名字", () => {
  assert.equal(parseReceiptDelivererName("code-builder 交活"), "code-builder");
  assert.equal(parseReceiptDelivererName("prd-engineer 的工单未完成"), undefined);
  assert.equal(parseReceiptDelivererName("frontend-design 的工单被中断"), undefined);
  assert.equal(parseReceiptDelivererName("交活"), undefined);
  assert.equal(parseReceiptDelivererName("随便一句话"), undefined);
});

test("selectForwardTargets：剔除交付方本人，其余全部保留", () => {
  const directory = [
    { name: "code-builder" },
    { name: "code-reviewer" },
    { name: "Code-Reviewer" },
  ];
  assert.deepEqual(
    selectForwardTargets(directory, "code-builder").map((agent) => agent.name),
    ["code-reviewer", "Code-Reviewer"],
  );
  assert.deepEqual(
    selectForwardTargets(directory, "code-reviewer").map((agent) => agent.name),
    ["code-builder"],
  );
  assert.equal(selectForwardTargets(directory, undefined).length, 3);
});

test("buildWorkOrderForwardTask：成果原文全文进任务，附言与缺省口径分别成文", () => {
  const withNote = buildWorkOrderForwardTask({
    delivererName: "code-builder",
    answer: "改了登录页，测试已跑绿。",
    note: "评审这份成果，给裁定",
  });
  assert.match(withNote, /老板转交：以下是同事 code-builder 交来的工作成果/);
  assert.match(withNote, /改了登录页，测试已跑绿。/);
  assert.match(withNote, /老板的后续要求：评审这份成果，给裁定/);

  const withoutNote = buildWorkOrderForwardTask({
    delivererName: "code-builder",
    answer: "成果",
    note: "   ",
  });
  assert.match(withoutNote, /拿不准下一步就问老板，不要自行发挥/);
});
