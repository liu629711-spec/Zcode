import assert from "node:assert/strict";
import test from "node:test";
import type { AgentSummary } from "@zcode/shared";
import {
  selectProjectAgentsForWorkspace,
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
    validateProjectAgentDraft({
      name: "code-reviewer",
      description: "审查",
      systemPrompt: "prompt",
    }),
    [],
  );
});

test("validateProjectAgentDraft：名称长度与字符集、必填项", () => {
  assert.deepEqual(validateProjectAgentDraft({ name: "ab", description: "d", systemPrompt: "p" }), [
    "nameLength",
  ]);
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

test("selectProjectAgentsForWorkspace：先按 scope 过滤，非 workspace 一律排除", () => {
  const agents = [
    { id: "1", scope: "built-in", path: "D:/repo/.zcode/agents/builtin.md" },
    { id: "2", scope: "user", path: "D:/repo/.zcode/agents/user.md" },
  ] as Pick<AgentSummary, "id" | "scope" | "path">[];
  assert.deepEqual(
    selectProjectAgentsForWorkspace(agents, "D:/repo").map((agent) => agent.id),
    [],
  );
});

test("selectProjectAgentsForWorkspace：path 前缀匹配，正反斜杠与盘符大小写容错", () => {
  const agents = [
    { id: "1", scope: "workspace", path: "D:\\repo\\.zcode\\agents\\reviewer.md" },
    { id: "2", scope: "workspace", path: "D:/repo/.zcode/agents/planner.md" },
    { id: "3", scope: "workspace", path: "d:/repo/.zcode/agents/casing.md" },
  ] as Pick<AgentSummary, "id" | "scope" | "path">[];
  assert.deepEqual(
    selectProjectAgentsForWorkspace(agents, "D:/repo").map((agent) => agent.id),
    ["1", "2", "3"],
  );
  assert.deepEqual(
    selectProjectAgentsForWorkspace(agents, "D:\\repo").map((agent) => agent.id),
    ["1", "2", "3"],
  );
});

test("selectProjectAgentsForWorkspace：他工作区的 agent 不归本工作区", () => {
  const agents = [
    { id: "1", scope: "workspace", path: "D:/other/.zcode/agents/foreign.md" },
    { id: "2", scope: "workspace", path: "D:/repo-evil/.zcode/agents/prefix-trap.md" },
    { id: "3", scope: "workspace", path: "D:/repo/.zcode/agents/own.md" },
  ] as Pick<AgentSummary, "id" | "scope" | "path">[];
  assert.deepEqual(
    selectProjectAgentsForWorkspace(agents, "D:/repo").map((agent) => agent.id),
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
  assert.deepEqual(toProjectAgentPersona({ ...base, memory: "project" } as AgentSummary), {
    name: "code-reviewer",
    systemPrompt: "你是代码审查员",
    memoryScope: "project",
  });
});

test("toProjectAgentPersona：档案的模型/工具/颜色随载荷走（G2 同一副面孔）", () => {
  const base = {
    id: "a1",
    name: "code-reviewer",
    systemPrompt: "你是代码审查员",
    scope: "workspace",
  } as Pick<AgentSummary, "id" | "name" | "systemPrompt" | "scope">;
  assert.deepEqual(
    toProjectAgentPersona({
      ...base,
      modelSelection: { providerId: "openai", modelId: "gpt-5" },
      tools: ["Read", "Grep"],
      disallowedTools: ["Bash"],
      color: "purple",
    } as AgentSummary),
    {
      name: "code-reviewer",
      systemPrompt: "你是代码审查员",
      modelSelection: { providerId: "openai", modelId: "gpt-5" },
      tools: ["Read", "Grep"],
      disallowedTools: ["Bash"],
      color: "purple",
    },
  );
});

test("toProjectAgentPersona：空工具数组不进载荷（= 继承全部工具），无色档案不带 color 键", () => {
  const base = {
    id: "a1",
    name: "code-reviewer",
    systemPrompt: "你是代码审查员",
    scope: "workspace",
    tools: [],
    disallowedTools: [],
  } as Pick<AgentSummary, "id" | "name" | "systemPrompt" | "scope" | "tools" | "disallowedTools">;
  assert.deepEqual(toProjectAgentPersona(base as AgentSummary), {
    name: "code-reviewer",
    systemPrompt: "你是代码审查员",
  });
});
