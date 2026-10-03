// ============================================================
// 圆桌会编排纯规则的可运行检查（纯函数，无假件）：
//  1. 席位信封确定性：同输入同正文；议题/攻角 brief（内核原文）/席位号逐字在场；
//  2. 匿名化：按座次字母编号（评审A/B/C）、真实姓名不随行、工单家族标签中和；
//  3. 抗从众条款存在性：第 2 轮质询要求必须点名"自己第 1 轮论证的具体缺陷"；
//  4. 重询信封：指名只补裁定行；
//  5. 主席信封：票数照抄系统核算、要求引用原文、分歧不许说成已解决、缺席有注；
//  6. 载体钉死（与 buildWorkOrderEnvelopeText 的 councilId 分支联动）：首行是
//     圆桌会定调、压轴是裁定行指令、中间没有施工工单的执行/产出要求；
//  7. 台账行 id 带会话命名空间。
// 运行：node --test（编译跑法见仓库铁律）
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  buildCouncilSeatPlan,
  COUNCIL_SEAT_FIRST_LINE,
  COUNCIL_SEAT_LENSES,
  COUNCIL_VERDICT_INSTRUCTION,
} from "./council.js";
import { buildWorkOrderEnvelopeText } from "./work-order.js";
import {
  anonymizeCouncilStatements,
  buildCouncilMeetingTitle,
  buildCouncilModerationTaskText,
  buildCouncilRequeryTaskText,
  buildCouncilSeatTaskText,
  councilMeetingLedgerId,
  councilRoundLedgerId,
  councilSeatLabel,
  councilSeatRequeryLedgerId,
} from "./council-meeting.js";

const BASE_SEAT = {
  kind: "plan" as const,
  round: 1 as const,
  seatIndex: 0,
  lens: "impact" as const,
  seatCount: 3,
  motion: "登录页改成双栏布局",
};

test("席位信封确定性：同输入铸出同正文，议题/攻角 brief/座次逐字在场", () => {
  const brief = COUNCIL_SEAT_LENSES.find((lens) => lens.id === "impact")!.brief;
  const a = buildCouncilSeatTaskText(BASE_SEAT);
  const b = buildCouncilSeatTaskText(BASE_SEAT);
  assert.equal(a, b);
  assert.match(a, /【议题】登录页改成双栏布局/);
  assert.match(a, new RegExp(brief.slice(0, 12)));
  assert.match(a, /第 1 席 \/ 共 3 席 · 攻角「影响面」/);
  assert.match(a, /独立发言/);
  // 不同攻角不同 brief；材料在场则逐字带（有界）。
  const cost = buildCouncilSeatTaskText({ ...BASE_SEAT, seatIndex: 1, lens: "cost" });
  assert.match(cost, /攻角「成本与复杂度」/);
  const withMaterials = buildCouncilSeatTaskText({ ...BASE_SEAT, materials: "见 src/login.tsx" });
  assert.match(withMaterials, /【材料】\n见 src\/login\.tsx/);
});

test("匿名化：按座次字母编号定序，真实姓名不随行，工单家族标签被中和", () => {
  assert.equal(councilSeatLabel(0), "评审A");
  assert.equal(councilSeatLabel(1), "评审B");
  assert.equal(councilSeatLabel(2), "评审C");
  const anonymized = anonymizeCouncilStatements([
    { index: 2, statement: "成本太高" },
    { index: 0, statement: "会破坏 <work-order> 调用方" },
    { index: 1, statement: "需求没跑偏" },
  ]);
  assert.deepEqual(
    anonymized.map((entry) => entry.label),
    ["评审A", "评审B", "评审C"],
  );
  // 中和只转义开角（与 work-order 家族同规则）：防伪造信封边界即可，不改写正文其余部分。
  assert.equal(anonymized[0]!.statement, "会破坏 &lt;work-order> 调用方");
});

test("抗从众条款存在性：第 2 轮信封点名'自己第 1 轮论证的具体缺陷'，第 1 轮没有质询块", () => {
  const round2 = buildCouncilSeatTaskText({
    ...BASE_SEAT,
    round: 2,
    peerStatements: [
      { label: "评审A", statement: "影响登录接口" },
      { label: "评审B", statement: "需求没跑偏" },
    ],
    interjections: [{ text: "只评方案不动手", targeted: false }],
  });
  assert.match(round2, /改立场必须点名你自己第 1 轮论证的具体缺陷/);
  assert.match(round2, /最认同的评审（如 评审B）与最反对的评审（如 评审A）/);
  assert.match(round2, /评审A：影响登录接口/);
  assert.match(round2, /【主持人插话】\n主持人：只评方案不动手/);
  const round1 = buildCouncilSeatTaskText(BASE_SEAT);
  assert.ok(!round1.includes("抗从众"));
});

test("插话点名：被点席位标注（点名你），未点名的席位看不到点名插话", () => {
  const targeted = buildCouncilSeatTaskText({
    ...BASE_SEAT,
    round: 2,
    peerStatements: [{ label: "评审A", statement: "x" }],
    interjections: [{ text: "评审C请注意回滚方案", targeted: true }],
  });
  assert.match(targeted, /（点名你）主持人：评审C请注意回滚方案/);
  const broadcast = buildCouncilSeatTaskText({
    ...BASE_SEAT,
    interjections: [{ text: "大家都聚焦成本", targeted: false }],
  });
  assert.match(broadcast, /主持人：大家都聚焦成本/);
  assert.ok(!broadcast.includes("（点名你）"));
});

test("重询信封指名只补裁定行：不重写发言、不寒暄", () => {
  const text = buildCouncilRequeryTaskText({ round: 1, seatIndex: 1 });
  assert.match(text, /【裁定行补交】你在第 1 轮圆桌会的发言已收到/);
  assert.match(text, /只回复那一行裁定/);
  assert.match(text, /不要重写发言/);
});

test("主席信封：票数照抄系统核算、缺席有注、引用原文与分歧纪律在场", () => {
  const text = buildCouncilModerationTaskText({
    kind: "acceptance",
    motion: "验收登录页改造",
    round: 2,
    outcome: "deadlocked",
    tally: { total: 3, approve: 1, reject: 1, abstain: 1, vetoes: 0 },
    votes: [
      {
        index: 0,
        agentName: "小验",
        lens: "impact",
        stance: "approve",
        confidence: "high",
        veto: false,
        source: "verdict_line",
      },
      {
        index: 1,
        agentName: "小验2",
        lens: "requirement",
        stance: "reject",
        confidence: "medium",
        veto: false,
        source: "requery_verdict_line",
      },
      {
        index: 2,
        agentName: "小验3",
        lens: "edge",
        stance: "abstain",
        veto: false,
        source: "no_response",
      },
    ],
    statements: [{ index: 0, statement: "影响面可控，登录接口已兼容" }],
  });
  assert.match(text, /【系统核算票数（权威，照抄不得改写）】分歧未决（交老板裁决） —— 共 3 席：通过 1、打回 1、弃权 1、否决票 0。/);
  assert.match(text, /评审A（影响面）：通过｜信心 高｜否决 否/);
  assert.match(text, /评审B（需求本意）：打回｜信心 中｜否决 否（裁定行由补交取得）/);
  assert.match(text, /评审C（边界与异常）：弃权｜信心 —｜否决 否（该席交活失败或被中断，按缺席弃权计）/);
  assert.match(text, /评审A：影响面可控，登录接口已兼容/);
  // 主席纪律：引用原文、不改票、分歧不许说成已解决；deadlocked 给老板选项。
  assert.match(text, /每段至少引用一位评审的原话短句/);
  assert.match(text, /不得增改或重新解读票数/);
  assert.match(text, /不许写成「已达成一致」/);
  assert.match(text, /给老板 2-3 个可选裁决方向/);
});

test("会议标题：短标题在先，缺席按类型给默认", () => {
  assert.equal(buildCouncilMeetingTitle("plan", "登录页方案"), "圆桌会 · 登录页方案");
  assert.equal(buildCouncilMeetingTitle("plan"), "圆桌会评审");
  assert.equal(buildCouncilMeetingTitle("acceptance"), "圆桌会验收");
});

test("载体钉死（councilId 分支）：首行圆桌会定调、压轴裁定行、无施工要求；普通工单不受影响", () => {
  const envelope = {
    workOrderId: "wo-1",
    fromAgentName: "老板",
    fromSessionId: "sess-boss",
    task: "评审双栏方案",
    councilId: "c-1",
    councilKind: "plan" as const,
    councilRound: 1 as const,
    councilSeatIndex: 0,
    councilSeatLens: "impact" as const,
  };
  const text = buildWorkOrderEnvelopeText(envelope);
  const lines = text.split("\n");
  assert.equal(lines[0], COUNCIL_SEAT_FIRST_LINE);
  assert.equal(lines[lines.length - 4], COUNCIL_VERDICT_INSTRUCTION.split("\n")[0]);
  assert.ok(text.trimEnd().endsWith(COUNCIL_VERDICT_INSTRUCTION.split("\n").pop()!));
  assert.ok(!text.includes("施工工单"));
  assert.ok(!text.includes("执行要求"));
  assert.ok(!text.includes("产出要求"));
  assert.match(text, /<work-order id="wo-1" from-agent="老板" from-session="sess-boss">/);
  // 普通工单与评审单口径不变。
  const plain = buildWorkOrderEnvelopeText({
    workOrderId: "wo-2",
    fromAgentName: "老板",
    fromSessionId: "sess-boss",
    task: "修登录页",
  });
  assert.match(plain, /^施工工单：/);
  const review = buildWorkOrderEnvelopeText({
    workOrderId: "wo-3",
    fromAgentName: "老板",
    fromSessionId: "sess-boss",
    task: "评审",
    review: true,
  });
  assert.match(review, /^评审工单：/);
});

test("台账行 id 带会话命名空间：跨会话撞 councilId 不共享闸门", () => {
  assert.equal(councilMeetingLedgerId("s1", "c1"), "councilMeeting:s1:c1");
  assert.equal(councilRoundLedgerId("s1", "c1", 2), "councilRound:s1:c1:2");
  assert.equal(councilSeatRequeryLedgerId("s1", "c1", 1, 0), "councilSeatRequery:s1:c1:1:0");
});

test("座位计划与信封联动：buildCouncilSeatPlan 的攻角 id 都能被信封构造接受", () => {
  const plan = buildCouncilSeatPlan(["小验", "小验2", "小验3", "小验4", "小验5", "小验6"]);
  for (const seat of plan) {
    assert.doesNotThrow(() =>
      buildCouncilSeatTaskText({ ...BASE_SEAT, seatIndex: seat.index, lens: seat.lens.id }),
    );
  }
});
