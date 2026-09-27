import assert from "node:assert/strict";
import test from "node:test";
import type { AgentSummary } from "@zcode/shared";
import {
  selectProjectAgents,
  toProjectAgentCreateConfig,
  toProjectAgentPersona,
  validateProjectAgentDraft,
} from "../src/WorkspaceSidebar/projectAgentsModel.js";

// ============================================================
// 侧栏驻场智能体的纯逻辑检查：表单校验 / 分组筛选 / 创建参数
// ============================================================
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/projectAgentsModel.test.ts

test("validateProjectAgentDraft：合法草稿无报错", () => {
  assert.deepEqual(
    validateProjectAgentDraft({ name: "code-reviewer", description: "审查", systemPrompt: "prompt" }),
    [],
  );
});

test("validateProjectAgentDraft：名称长度与字符集、必填项", () => {
  assert.deepEqual(
    validateProjectAgentDraft({ name: "ab", description: "d", systemPrompt: "p" }),
    ["nameLength"],
  );
  assert.deepEqual(
    validateProjectAgentDraft({ name: "a".repeat(51), description: "d", systemPrompt: "p" }),
    ["nameLength"],
  );
  assert.deepEqual(
    validateProjectAgentDraft({ name: "非法名称", description: "d", systemPrompt: "p" }),
    ["nameCharacters"],
  );
  assert.deepEqual(
    validateProjectAgentDraft({ name: "abc", description: " ", systemPrompt: "p" }),
    ["descriptionRequired"],
  );
  assert.deepEqual(
    validateProjectAgentDraft({ name: "abc", description: "d", systemPrompt: " " }),
    ["promptRequired"],
  );
});

test("selectProjectAgents：只保留 workspace 作用域", () => {
  const agents = [
    { id: "1", scope: "built-in" },
    { id: "2", scope: "user" },
    { id: "3", scope: "workspace" },
  ] as Pick<AgentSummary, "id" | "scope">[];
  assert.deepEqual(
    selectProjectAgents(agents).map((agent) => agent.id),
    ["3"],
  );
});

test("toProjectAgentCreateConfig：trim 字段并固定 memory=project", () => {
  const config = toProjectAgentCreateConfig({
    name: "  reviewer  ",
    description: " desc ",
    systemPrompt: " prompt ",
  });
  assert.equal(config.name, "reviewer");
  assert.equal(config.description, "desc");
  assert.equal(config.systemPrompt, "prompt");
  // 驻场语义钉在这里：片 1 的 memory 序列化落地后即按此值写入 markdown。
  assert.equal((config as { memory?: string }).memory, "project");
});

test("toProjectAgentPersona：persona 载荷带名称/提示词/记忆 scope", () => {
  const base = {
    id: "a1",
    name: "code-reviewer",
    systemPrompt: "你是代码审查员",
    scope: "workspace",
  } as Pick<AgentSummary, "id" | "name" | "systemPrompt" | "scope">;
  assert.deepEqual(toProjectAgentPersona(base as AgentSummary), {
    name: "code-reviewer",
    systemPrompt: "你是代码审查员",
  });
  assert.deepEqual(
    toProjectAgentPersona({ ...base, memory: "project" } as AgentSummary),
    { name: "code-reviewer", systemPrompt: "你是代码审查员", memoryScope: "project" },
  );
});
