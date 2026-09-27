import assert from "node:assert/strict";
import test from "node:test";
import type { AgentSummary } from "@zcode/shared";
import {
  applyPersonaChatBadges,
  buildAgentMemoryDirectoryHint,
  getPersonaChatBadge,
  resolveWorkspaceProjectAgentReachability,
  selectProjectAgentsForWorkspace,
  toProjectAgentCreateConfig,
  toProjectAgentPersona,
  toProjectAgentPersonaFromDraft,
  toProjectAgentUpdateConfig,
  validateProjectAgentDraft,
  type PersonaChatBadge,
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

test("toProjectAgentPersonaFromDraft：trim、固定 project scope，不带模型/工具/颜色键", () => {
  assert.deepEqual(
    toProjectAgentPersonaFromDraft({
      name: "  reviewer  ",
      description: "desc",
      systemPrompt: " 你是代码审查员 ",
    }),
    { name: "reviewer", systemPrompt: "你是代码审查员", memoryScope: "project" },
  );
});

test("resolveWorkspaceProjectAgentReachability：bound 本地只开本地 tab，bound 远程只开同路径 tab", () => {
  const base = {
    boundWorkspacePath: "D:/repo",
    tabWorkspacePath: "D:/repo",
  };
  // bound 本地：本地 tab 可达，远程 tab 不可达（防止经本机服务把目录建到错误的机器上）。
  assert.equal(resolveWorkspaceProjectAgentReachability({ ...base }), true);
  assert.equal(
    resolveWorkspaceProjectAgentReachability({ ...base, tabRemoteSessionId: "remote-1" }),
    false,
  );
  // bound 远程：只有与 bound 同路径的 tab 可达，其它（含本地）一律不可达。
  const remoteBound = { ...base, boundWorkspaceRemoteSessionId: "remote-1" };
  assert.equal(resolveWorkspaceProjectAgentReachability(remoteBound), true);
  assert.equal(
    resolveWorkspaceProjectAgentReachability({
      ...remoteBound,
      tabWorkspacePath: "D:/other",
    }),
    false,
  );
  assert.equal(
    resolveWorkspaceProjectAgentReachability({ ...remoteBound, tabRemoteSessionId: undefined }),
    true,
  );
});

test("applyPersonaChatBadges：命中行挂徽章，未命中行保持原引用，空登记不换引用", () => {
  const row = (taskId: string) => ({ taskId, title: `t-${taskId}` });
  const a = row("a");
  const b = row("b");
  const badge: PersonaChatBadge = { name: "code-reviewer", color: "purple" };

  const badged = applyPersonaChatBadges([a, b], new Map([["a", badge]]));
  assert.notEqual(badged[0], b);
  assert.deepEqual((badged[0] as { agentPersona?: PersonaChatBadge }).agentPersona, badge);
  assert.equal(badged[1], b, "未命中行必须保持原引用（行级 memo 靠引用判等）");
  assert.equal((badged[1] as { agentPersona?: PersonaChatBadge }).agentPersona, undefined);

  // 空登记：数组内容等价（applyPersonaChatBadges 允许返回同内容新数组，由调用方 useMemo 兜引用稳定）。
  const untouched = applyPersonaChatBadges([a, b], new Map());
  assert.deepEqual(untouched, [a, b]);
  assert.equal(untouched[0], a);

  // 登记里有列表外的 taskId：忽略，不炸。
  assert.deepEqual(
    applyPersonaChatBadges([a], new Map([["ghost", badge]])).map((item) => item.taskId),
    ["a"],
  );
});

test("toProjectAgentUpdateConfig：三框替换自草稿，其余档案字段原样带回（updateAgent 整文件重写）", () => {
  const agent = {
    id: "agent-1",
    name: "old-name",
    description: "old desc",
    systemPrompt: "old prompt",
    color: "purple",
    modelSelection: { provider: "zcode", model: "glm-4.7" },
    tools: ["Read", "Grep"],
    disallowedTools: [],
    skills: ["review"],
    permissionMode: "plan",
    memory: "project",
    maxTurns: 20,
    background: false,
    injectAgentsMd: true,
    path: "D:/repo/.zcode/agents/old-name.md",
    scope: "workspace",
    source: "user",
    enabled: true,
  } as AgentSummary;

  assert.deepEqual(
    toProjectAgentUpdateConfig(agent, {
      name: " new-name ",
      description: "新介绍",
      systemPrompt: "新提示词",
    }),
    {
      name: "new-name",
      description: "新介绍",
      systemPrompt: "新提示词",
      color: "purple",
      modelSelection: { provider: "zcode", model: "glm-4.7" },
      tools: ["Read", "Grep"],
      skills: ["review"],
      permissionMode: "plan",
      memory: "project",
      maxTurns: 20,
      background: false,
      injectAgentsMd: true,
    },
  );
});

test("toProjectAgentUpdateConfig：空列表与缺席字段不落盘，记忆范围缺省 project", () => {
  const agent = {
    id: "agent-2",
    name: "bare",
    description: "d",
    systemPrompt: "p",
    path: "D:/repo/.zcode/agents/bare.md",
    scope: "workspace",
    source: "user",
    enabled: true,
  } as AgentSummary;

  const config = toProjectAgentUpdateConfig(agent, {
    name: "bare",
    description: "d",
    systemPrompt: "p",
  });
  assert.deepEqual(config, { name: "bare", description: "d", systemPrompt: "p", memory: "project" });
  assert.equal("tools" in config, false);
  assert.equal("disallowedTools" in config, false);
  assert.equal("mcpServers" in config, false);
});

test("buildAgentMemoryDirectoryHint：project/local 给出项目内路径，user 返回 null，反斜杠归一", () => {
  const agent = (memory?: string) =>
    ({ name: "code-reviewer", ...(memory ? { memory } : {}) }) as Pick<
      AgentSummary,
      "name" | "memory"
    >;

  assert.equal(
    buildAgentMemoryDirectoryHint(agent(), "D:/repo"),
    "D:/repo/.zcode/agent-memory/code-reviewer/",
  );
  assert.equal(
    buildAgentMemoryDirectoryHint(agent("project"), "D:\\repo"),
    "D:/repo/.zcode/agent-memory/code-reviewer/",
  );
  assert.equal(
    buildAgentMemoryDirectoryHint(agent("local"), "D:/repo"),
    "D:/repo/.zcode/agent-memory-local/code-reviewer/",
  );
  assert.equal(buildAgentMemoryDirectoryHint(agent("user"), "D:/repo"), null);
});

test("getPersonaChatBadge：读 agentPersona 内联标记，普通会话行无徽章", () => {
  const plain = { taskId: "t1" } as Parameters<typeof getPersonaChatBadge>[0];
  assert.equal(getPersonaChatBadge(plain), undefined);

  const badged = {
    taskId: "t2",
    agentPersona: { name: "code-reviewer" },
  } as Parameters<typeof getPersonaChatBadge>[0];
  assert.deepEqual(getPersonaChatBadge(badged), { name: "code-reviewer" });
});
