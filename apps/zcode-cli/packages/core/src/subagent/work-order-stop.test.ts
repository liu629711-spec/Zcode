// ============================================================
// 工单停机守卫的可运行检查（work-order-stop.ts 纯函数）。
// 运行（本机 tsx 挂死，用 tsc 编译 + node --test，outDir 必须在仓库内）：
//   node node_modules/typescript/bin/tsc apps/zcode-cli/packages/core/src/subagent/work-order-stop.test.ts --outDir .zcode/workflow-drafts/gate-out --rootDir . --module nodenext --moduleResolution nodenext --target es2023 --skipLibCheck --strict --types node
//   node --test .zcode/workflow-drafts/gate-out/apps/zcode-cli/packages/core/src/subagent/work-order-stop.test.js
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  assessWorkOrderStop,
  isAckOnlyWorkOrderReply,
  WORK_ORDER_NO_ACTION_ERROR,
  WORK_ORDER_NUDGE_TEXT,
} from "./work-order-stop.js";

test("寒暄判定：真机观测过的两种弱模型回样都算寒暄", () => {
  assert.equal(isAckOnlyWorkOrderReply("随时可以开工"), true);
  assert.equal(isAckOnlyWorkOrderReply("随时可以开工，等您的指令！"), true);
  assert.equal(isAckOnlyWorkOrderReply("在的~"), true);
  assert.equal(isAckOnlyWorkOrderReply("收到，老板"), true);
  assert.equal(isAckOnlyWorkOrderReply("好的，我已就位。"), true);
  assert.equal(isAckOnlyWorkOrderReply("OK"), true);
});

test("寒暄判定：空回复算寒暄；真正内容不算", () => {
  assert.equal(isAckOnlyWorkOrderReply(""), true);
  assert.equal(isAckOnlyWorkOrderReply("   \n  "), true);
  assert.equal(isAckOnlyWorkOrderReply("登录表单组件已完成，文件在 src/components/LoginForm.tsx"), false);
  assert.equal(isAckOnlyWorkOrderReply("裁定：通过｜信心：高｜否决：否——理由如下……"), false);
});

test("寒暄判定：带寒暄词的真交付不误伤（30 字上界）", () => {
  // 短但含"开工"的完整成果罕见；超 30 字直接放行。
  assert.equal(
    isAckOnlyWorkOrderReply(
      "开工注意事项共三条：一、先跑测试；二、再看 diff；三、最后写交接单，缺一不可。",
    ),
    false,
  );
});

test("裁决：动过手（有工具调用）一律放行，不管文本像什么", () => {
  assert.equal(
    assessWorkOrderStop({ modelResponse: "随时可以开工", toolCallCount: 3 }),
    "deliver",
  );
});

test("裁决：评审单豁免（只评审不动手是纪律不是病）", () => {
  assert.equal(
    assessWorkOrderStop({ modelResponse: "收到", toolCallCount: 0, review: true }),
    "deliver",
  );
});

test("裁决：首次寒暄补一脚，再犯判失败", () => {
  assert.equal(
    assessWorkOrderStop({ modelResponse: "在的~", toolCallCount: 0 }),
    "nudge",
  );
  assert.equal(
    assessWorkOrderStop({ modelResponse: "在的~", toolCallCount: 0, alreadyNudged: true }),
    "fail",
  );
  // 补射后真正动手/交付：放行。
  assert.equal(
    assessWorkOrderStop({ modelResponse: "交付：登录表单已完成", toolCallCount: 0, alreadyNudged: true }),
    "deliver",
  );
});

test("文案钉：提醒给足「交付：」逃生口；失败原因说人话", () => {
  assert.equal(WORK_ORDER_NUDGE_TEXT.includes("交付："), true);
  assert.equal(WORK_ORDER_NUDGE_TEXT.includes("工单即任务"), true);
  assert.equal(WORK_ORDER_NO_ACTION_ERROR.includes("可打回重派"), true);
});
