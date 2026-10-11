import assert from "node:assert/strict";
import test from "node:test";
import {
  parseSubagentMarkdown,
  serializeSubagentMarkdown,
} from "../src/subagents/subagentMarkdown.js";

// 组长开关（组长制批 2026-10-12）的档案读写往返：
// teamLead: true 落盘/解析保真；缺省 = false 语义，不写多余行。

test("teamLead 写入/解析往返；缺省不带行、解析为 undefined", () => {
  const withLead = serializeSubagentMarkdown({
    name: "组长一号",
    description: "带头干活",
    systemPrompt: "先摸清目录再动手。",
    teamLead: true,
  });
  assert.ok(withLead.includes("teamLead: true"));
  const parsedLead = parseSubagentMarkdown({
    content: withLead,
    path: "/agents/lead.md",
    scope: "user",
  });
  assert.equal(parsedLead.agent?.teamLead, true);
  assert.equal(parsedLead.agent?.name, "组长一号");
  assert.equal(parsedLead.agent?.systemPrompt, "先摸清目录再动手。");

  const withoutLead = serializeSubagentMarkdown({
    name: "普通员工",
    description: "d",
    systemPrompt: "p",
  });
  assert.ok(!withoutLead.includes("teamLead"));
  const parsedPlain = parseSubagentMarkdown({
    content: withoutLead,
    path: "/agents/plain.md",
    scope: "user",
  });
  assert.equal(parsedPlain.agent?.teamLead, undefined);

  // 手写档案（老格式无此行 / 显式 false）都解析得出，不炸。
  const handwritten = parseSubagentMarkdown({
    content: '---\nname: "老档案"\ndescription: "d"\nteamLead: false\n---\n正文',
    path: "/agents/old.md",
    scope: "user",
  });
  assert.equal(handwritten.agent?.teamLead, false);
});
