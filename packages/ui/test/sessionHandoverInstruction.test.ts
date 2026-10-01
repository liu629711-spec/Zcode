import assert from "node:assert/strict";
import { test } from "node:test";
import {
  buildHandoverFilePath,
  buildHandoverFirstInput,
  buildHandoverGenerationInstruction,
} from "../src/v4/sessionHandoverInstruction.js";

test("handover file path normalizes trailing separators", () => {
  assert.equal(
    buildHandoverFilePath("D:\\work\\demo\\", "sess_abc"),
    "D:\\work\\demo/.zcode/handovers/sess_abc.md",
  );
  assert.equal(
    buildHandoverFilePath("D:/work/demo", "sess_abc"),
    "D:/work/demo/.zcode/handovers/sess_abc.md",
  );
});

test("generation instruction pins the ten-section template and file target", () => {
  const instruction = buildHandoverGenerationInstruction("D:/work/demo", "sess_abc");
  for (const section of [
    "三句话老板版",
    "老板的目标与原话要点",
    "已经干完的",
    "干到一半的",
    "还没干的",
    "死路与陷阱",
    "关键决定及理由",
    "关键文件与坐标",
    "与记忆柜的分工",
    "红线",
  ]) {
    assert.ok(instruction.includes(section), `missing section: ${section}`);
  }
  assert.ok(instruction.includes("D:/work/demo/.zcode/handovers/sess_abc.md"));
  // 必须指名写文件而不是把全文吐进聊天：接班会话靠引用文件防搬运失真。
  assert.ok(instruction.includes("Write"));
  assert.ok(instruction.includes("3000 字"));
});

test("first input tells the successor to read the file, not carry the text", () => {
  const firstInput = buildHandoverFirstInput("D:/work/demo", "sess_abc");
  assert.ok(firstInput.includes("D:/work/demo/.zcode/handovers/sess_abc.md"));
  assert.ok(firstInput.includes("Read"));
  assert.ok(!firstInput.includes("三句话老板版"));
});

test("接班首条消息带两条铁规矩：交接单缺失即停 + 在跑单勿默认重做（audit 2026-10-01）", () => {
  const firstInput = buildHandoverFirstInput("D:/work/demo", "sess_abc");
  assert.ok(firstInput.includes("换班没有完成"), "交接单缺失必须停下上报");
  assert.ok(firstInput.includes("不要默认重做"), "在跑的派单不得默认重做");
  assert.ok(firstInput.includes(".zcode/handovers/sess_abc.md"), "交接单路径仍然指向旧会话");
});
