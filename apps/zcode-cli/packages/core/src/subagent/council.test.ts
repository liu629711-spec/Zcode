// ============================================================
// 圆桌会内核协议的可运行检查（纯函数，无假件）：
//  1. 席位分配确定性：同名单同座次；空名单/空白名过滤；多于攻角数轮转复用；
//  2. 裁定行解析：标准行、全角分隔符、容忍空格；全文取最后一个匹配；
//     缺行/槽位越界（"同意""较大"等变体）→ undefined，散文不当票；
//  3. 算票：三态计数与否决计数分得开；
//  4. 决议：首轮全票同立场早退（通过/打回都省第二轮）；首轮分裂进质询轮；
//     终轮任一否决即否决；通过须同时压过打回与弃权；平票/弃权拖垮 = 分歧未决。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/subagent/council.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  buildCouncilSeatPlan,
  countCouncilVotes,
  COUNCIL_SEAT_LENSES,
  COUNCIL_VERDICT_INSTRUCTION,
  parseCouncilVerdictLine,
  resolveCouncilDecision,
} from "./council.js";

test("席位分配：同名单铸出同座次，空白名被过滤", () => {
  const plan = buildCouncilSeatPlan(["小验", " xiaoyan2 ", "小验3", ""]);
  assert.equal(plan.length, 3);
  assert.deepEqual(
    plan.map((seat) => seat.agentName),
    ["小验", "xiaoyan2", "小验3"],
  );
  assert.deepEqual(
    plan.map((seat) => seat.lens.id),
    ["impact", "requirement", "edge"],
  );
  const again = buildCouncilSeatPlan(["小验", "xiaoyan2", "小验3"]);
  assert.deepEqual(again, plan.slice(0, 3));
});

test("席位分配：空名单给空座次，超出攻角数轮转复用", () => {
  assert.deepEqual(buildCouncilSeatPlan([]), []);
  assert.deepEqual(buildCouncilSeatPlan(["  ", ""]), []);
  const plan = buildCouncilSeatPlan(["甲", "乙", "丙", "丁", "戊", "己"]);
  assert.equal(plan.length, 6);
  assert.equal(plan[0].lens.id, plan[5].lens.id);
  assert.equal(plan[4].lens.id, COUNCIL_SEAT_LENSES[4].id);
});

test("裁定行解析：标准行、全角分隔、容忍空格", () => {
  assert.deepEqual(parseCouncilVerdictLine("裁定：通过｜信心：高｜否决：否"), {
    stance: "approve",
    confidence: "high",
    veto: false,
  });
  assert.deepEqual(
    parseCouncilVerdictLine("裁定: 打回 | 信心: 低 | 否决: 是"),
    { stance: "reject", confidence: "low", veto: true },
  );
  assert.deepEqual(
    parseCouncilVerdictLine("裁定：弃权｜信心：中｜否决：否"),
    { stance: "abstain", confidence: "medium", veto: false },
  );
});

test("裁定行解析：全文取最后一个匹配；缺行与变体不给票", () => {
  const text = [
    "我先支持这个方案。",
    "裁定：通过｜信心：高｜否决：否",
    "但听完质询后我改主意了。",
    "裁定：打回｜信心：中｜否决：是",
  ].join("\n");
  assert.deepEqual(parseCouncilVerdictLine(text), {
    stance: "reject",
    confidence: "medium",
    veto: true,
  });
  assert.equal(parseCouncilVerdictLine("我同意这个方案，很有信心，没有否决意见。"), undefined);
  assert.equal(parseCouncilVerdictLine("裁定：同意｜信心：较大｜否决：没有"), undefined);
  assert.equal(parseCouncilVerdictLine("裁定：通过｜信心：高"), undefined);
  assert.equal(parseCouncilVerdictLine(""), undefined);
});

test("算票：三态分开计，否决独立计数", () => {
  const tally = countCouncilVotes([
    { stance: "approve", confidence: "high", veto: false },
    { stance: "approve", confidence: "medium", veto: false },
    { stance: "reject", confidence: "high", veto: true },
    { stance: "abstain", confidence: "low", veto: false },
  ]);
  assert.deepEqual(tally, { total: 4, approve: 2, reject: 1, abstain: 1, vetoes: 1 });
});

test("决议：首轮全票同立场早退，分裂进质询轮", () => {
  const unanimousApprove = { total: 3, approve: 3, reject: 0, abstain: 0, vetoes: 0 };
  assert.deepEqual(resolveCouncilDecision(unanimousApprove, 1), { outcome: "approved" });
  const unanimousReject = { total: 3, approve: 0, reject: 3, abstain: 0, vetoes: 0 };
  assert.deepEqual(resolveCouncilDecision(unanimousReject, 1), { outcome: "rejected" });
  const split = { total: 3, approve: 2, reject: 1, abstain: 0, vetoes: 0 };
  assert.deepEqual(resolveCouncilDecision(split, 1), { outcome: "deliberate", nextRound: 2 });
  const approveWithAbstain = { total: 3, approve: 2, reject: 0, abstain: 1, vetoes: 0 };
  assert.deepEqual(resolveCouncilDecision(approveWithAbstain, 1), {
    outcome: "deliberate",
    nextRound: 2,
  });
  assert.deepEqual(
    resolveCouncilDecision({ total: 0, approve: 0, reject: 0, abstain: 0, vetoes: 0 }, 1),
    { outcome: "deliberate", nextRound: 2 },
  );
});

test("决议：终轮按全体过半判定，否决一票封杀", () => {
  const vetoed = { total: 4, approve: 3, reject: 1, abstain: 0, vetoes: 1 };
  assert.deepEqual(resolveCouncilDecision(vetoed, 2), { outcome: "rejected" });
  const majority = { total: 4, approve: 3, reject: 1, abstain: 0, vetoes: 0 };
  assert.deepEqual(resolveCouncilDecision(majority, 2), { outcome: "approved" });
  const plurality = { total: 4, approve: 2, reject: 1, abstain: 1, vetoes: 0 };
  assert.deepEqual(resolveCouncilDecision(plurality, 2), { outcome: "deadlocked" });
  const tie = { total: 4, approve: 2, reject: 2, abstain: 0, vetoes: 0 };
  assert.deepEqual(resolveCouncilDecision(tie, 2), { outcome: "deadlocked" });
  const rejectMajority = { total: 4, approve: 0, reject: 3, abstain: 1, vetoes: 0 };
  assert.deepEqual(resolveCouncilDecision(rejectMajority, 2), { outcome: "rejected" });
  const rejectPlurality = { total: 4, approve: 1, reject: 2, abstain: 1, vetoes: 0 };
  assert.deepEqual(resolveCouncilDecision(rejectPlurality, 2), { outcome: "deadlocked" });
  const abstainHeavy = { total: 4, approve: 1, reject: 0, abstain: 3, vetoes: 0 };
  assert.deepEqual(resolveCouncilDecision(abstainHeavy, 2), { outcome: "deadlocked" });
  assert.deepEqual(
    resolveCouncilDecision({ total: 0, approve: 0, reject: 0, abstain: 0, vetoes: 0 }, 2),
    { outcome: "deadlocked" },
  );
});

test("裁定指令：钉死三个槽位的合法取值与退回后果", () => {
  for (const slot of ["通过/打回/弃权", "高/中/低", "是/否"]) {
    assert.ok(COUNCIL_VERDICT_INSTRUCTION.includes(slot));
  }
  assert.ok(COUNCIL_VERDICT_INSTRUCTION.includes("退回重交"));
});
