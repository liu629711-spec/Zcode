import assert from "node:assert/strict";
import test from "node:test";
import type { SubAgentConfig } from "@zcode/shared";
import {
  parseSubagentMarkdown,
  serializeSubagentMarkdown,
} from "../src/subagents/subagentMarkdown.js";

const MD_WITH_MEMORY = `---
name: "rememberer"
description: "Keeps project notes"
memory: project
---

You remember things.
`;

const MD_WITHOUT_MEMORY = MD_WITH_MEMORY.replace("memory: project\n", "");

function parseAgent(content: string) {
  return parseSubagentMarkdown({ content, path: "/tmp/rememberer.md", scope: "user" });
}

test("parse reads memory scope from frontmatter", () => {
  const { agent, diagnostic } = parseAgent(MD_WITH_MEMORY);
  assert.equal(diagnostic, undefined);
  assert.equal(agent?.memory, "project");
});

test("parse keeps memory through a serialize round-trip", () => {
  for (const memory of ["user", "project", "local"] as const) {
    const config: SubAgentConfig = {
      name: "rememberer",
      description: "Keeps project notes",
      systemPrompt: "You remember things.",
      memory,
    };
    const serialized = serializeSubagentMarkdown(config);
    const { agent, diagnostic } = parseAgent(serialized);
    assert.equal(diagnostic, undefined);
    assert.equal(agent?.memory, memory);
  }
});

test("parse ignores an invalid memory scope without throwing", () => {
  const invalid = MD_WITH_MEMORY.replace("memory: project", 'memory: "yes"');
  const { agent, diagnostic } = parseAgent(invalid);
  assert.equal(diagnostic, undefined);
  assert.equal(agent?.memory, undefined);
});

test("serialize only writes the memory line when memory is set", () => {
  const withMemory = serializeSubagentMarkdown({
    name: "rememberer",
    description: "Keeps project notes",
    systemPrompt: "You remember things.",
    memory: "project",
  });
  assert.ok(withMemory.includes("memory: project"));

  const withoutMemory = serializeSubagentMarkdown({
    name: "rememberer",
    description: "Keeps project notes",
    systemPrompt: "You remember things.",
  });
  assert.ok(!withoutMemory.includes("memory"));

  // 旧 agent 文件（无 memory 字段）解析照旧，再序列化也不凭空多出 memory 行。
  const legacy = parseAgent(MD_WITHOUT_MEMORY);
  assert.equal(legacy.agent?.memory, undefined);
  assert.ok(!serializeSubagentMarkdown(legacy.agent!).includes("memory"));
});
