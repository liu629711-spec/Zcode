import assert from "node:assert/strict";
import test from "node:test";
import type { AgentSummary } from "@zcode/shared";
import { resolveAgentMemoryRoot } from "@zcode/shared/node";
import {
  applyDerivedPersonaChatBadges,
  applyPersonaChatBadges,
  buildAgentMemoryDirectoryHint,
  buildRenamedPersonaTitle,
  buildRetitledPersonaTitle,
  derivePersonaChatBadge,
  findLatestPersonaChatRow,
  findLatestArchivedPersonaChatRow,
  findPersonaRowIdsByTitlePrefix,
  getPersonaChatBadge,
  isPersonaChatRowOfRenamedAgent,
  groupPersonaBadgedTaskItems,
  personaChatRenameDraft,
  resolvePersonaChatBadgeForTask,
  resolveWorkspaceProjectAgentReachability,
  restorePersonaTitlePrefix,
  refreshPersonaChatBadgeFromDirectory,
  selectProjectAgentsForWorkspace,
  selectWorkspaceRosterAgents,
  stripPersonaTitlePrefix,
  toProjectAgentCreateConfig,
  toProjectAgentPersona,
  toProjectAgentUpdateConfig,
  toPersonaChatBadge,
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

// ── 名册合并（身份轴终局 §九②）：项目档 ∪ 全局员工，处处可见 ──────────────

function rosterAgent(id: string, overrides: Partial<AgentSummary>): AgentSummary {
  return {
    id,
    name: id,
    description: `${id} 的活`,
    systemPrompt: "p",
    path: `D:/machine/${id}.md`,
    scope: "user",
    source: "user",
    enabled: true,
    ...overrides,
  } as AgentSummary;
}

test("selectWorkspaceRosterAgents：全局员工并入每个工作区名册，项目档存量照旧", () => {
  const agents = [
    rosterAgent("global-doc-writer", { path: "C:/Users/me/.zcode/agents/doc-writer.md" }),
    rosterAgent("proj-reviewer", {
      scope: "workspace",
      source: "user",
      path: "D:/repo/.zcode/agents/reviewer.md",
    }),
    rosterAgent("proj-planner", {
      scope: "workspace",
      source: "user",
      path: "D:/other/.zcode/agents/planner.md",
    }),
  ];
  const roster = selectWorkspaceRosterAgents(agents, "D:/repo");
  // 本工作区项目档 + 全局员工；他工作区的项目档不进。
  assert.deepEqual(roster.map((agent) => agent.id).sort(), ["global-doc-writer", "proj-reviewer"]);
});

test("selectWorkspaceRosterAgents：同名 user 档与项目档是两位员工，互不覆盖", () => {
  const agents = [
    rosterAgent("user:doc-writer", {
      name: "doc-writer",
      path: "C:/Users/me/.zcode/agents/doc-writer.md",
    }),
    rosterAgent("ws:doc-writer", {
      name: "doc-writer",
      scope: "workspace",
      source: "user",
      path: "D:/repo/.zcode/agents/doc-writer.md",
    }),
  ];
  const roster = selectWorkspaceRosterAgents(agents, "D:/repo");
  assert.equal(roster.length, 2);
});

test("selectWorkspaceRosterAgents：插件档与禁用的 user 档不进名册", () => {
  const agents = [
    rosterAgent("plugin-pptx", { source: "plugin" }),
    rosterAgent("disabled-worker", { enabled: false }),
    rosterAgent("live-worker"),
  ];
  assert.deepEqual(
    selectWorkspaceRosterAgents(agents, "D:/repo").map((agent) => agent.id),
    ["live-worker"],
  );
});

test("selectWorkspaceRosterAgents：合并后按名字重排，不因来源分家而乱序", () => {
  const agents = [
    rosterAgent("ws-zeta", {
      name: "zeta",
      scope: "workspace",
      source: "user",
      path: "D:/repo/.zcode/agents/zeta.md",
    }),
    rosterAgent("global-alpha", { name: "alpha" }),
    rosterAgent("ws-mike", {
      name: "mike",
      scope: "workspace",
      source: "user",
      path: "D:/repo/.zcode/agents/mike.md",
    }),
  ];
  assert.deepEqual(
    selectWorkspaceRosterAgents(agents, "D:/repo").map((agent) => agent.name),
    ["alpha", "mike", "zeta"],
  );
});

test("toProjectAgentCreateConfig：trim 字段并缺省 memory=project", () => {
  const config = toProjectAgentCreateConfig({
    name: "  reviewer  ",
    description: " desc ",
    systemPrompt: " prompt ",
  });
  assert.equal(config.name, "reviewer");
  assert.equal(config.description, "desc");
  assert.equal(config.systemPrompt, "prompt");
  // 驻场语义钉在这里：手工建档仍走项目档案（收编另有通道）。
  assert.equal((config as { memory?: string }).memory, "project");
});

test("toProjectAgentCreateConfig：显式 memory 覆盖缺省（预置班底装用户级=随身本）", () => {
  // 身份轴终局 §九：班底装进用户级档案目录，memory=user 随身本跟着人走。
  const config = toProjectAgentCreateConfig(
    { name: "doc-writer", description: "文档文员", systemPrompt: "p" },
    { memory: "user" },
  );
  assert.equal((config as { memory?: string }).memory, "user");
  assert.equal((config as { memory?: string }).name, "doc-writer");
  // 非法 scope 编译期就被 AgentMemoryScope 拦住；缺省路径必须不受影响。
  assert.equal(
    (toProjectAgentCreateConfig({ name: "doc-writer", description: "d", systemPrompt: "p" }) as {
      memory?: string;
    }).memory,
    "project",
  );
});

test("toProjectAgentPersona：persona 载荷带名称/提示词/记忆 scope，档案缺 memory 缺省 project", () => {
  const base = {
    id: "a1",
    name: "code-reviewer",
    systemPrompt: "你是代码审查员",
    scope: "workspace",
  } as Pick<AgentSummary, "id" | "name" | "systemPrompt" | "scope">;
  // 设置页三框建档等路径的档案可能没有 memory 字段：载荷必须缺省 project。
  // core 以 persona.memory 在场为记忆注入/补工具的门，缺席 = 记事本成摆设。
  assert.deepEqual(toProjectAgentPersona(base as AgentSummary), {
    name: "code-reviewer",
    systemPrompt: "你是代码审查员",
    memoryScope: "project",
  });
  assert.deepEqual(toProjectAgentPersona({ ...base, memory: "project" } as AgentSummary), {
    name: "code-reviewer",
    systemPrompt: "你是代码审查员",
    memoryScope: "project",
  });
  assert.deepEqual(toProjectAgentPersona({ ...base, memory: "local" } as AgentSummary), {
    name: "code-reviewer",
    systemPrompt: "你是代码审查员",
    memoryScope: "local",
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
      memoryScope: "project",
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
    memoryScope: "project",
  });
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

test("toProjectAgentUpdateConfig：模型选择随表单走——带了就替换，键在场为空=清回继承（2026-09-29 指派模型）", () => {
  const agent = {
    id: "agent-3",
    name: "with-model",
    description: "d",
    systemPrompt: "p",
    modelSelection: { providerId: "provider-a", modelId: "old-model" },
    path: "D:/repo/.zcode/agents/with-model.md",
    scope: "workspace",
    source: "user",
    enabled: true,
  } as AgentSummary;

  // 表单换了模型 → 用表单的。
  const replaced = toProjectAgentUpdateConfig(agent, {
    name: "with-model",
    description: "d",
    systemPrompt: "p",
    modelSelection: { providerId: "provider-a", modelId: "new-model" },
  });
  assert.deepEqual(replaced.modelSelection, { providerId: "provider-a", modelId: "new-model" });

  // 表单键在场但为空 = 用户清空选择 → 模型落空（继承默认），不得回填旧值。
  const cleared = toProjectAgentUpdateConfig(agent, {
    name: "with-model",
    description: "d",
    systemPrompt: "p",
    modelSelection: undefined,
  });
  assert.equal("modelSelection" in cleared, false);
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

test("漂移钉住：UI 记事本提示与 shared 目录推导（core/面板同源）输出一致", () => {
  // UI 的提示是纯字符串拼接，不引 node 模块；这里与 @zcode/shared/node 的唯一实现
  // 对同一输入对账，任何一侧改推导（目录名/key 清洗/分隔符）都会在这里爆。
  for (const scope of ["project", "local"] as const) {
    const sharedRoot = resolveAgentMemoryRoot({
      agentName: "code-reviewer",
      scope,
      storageRoot: "",
      workspaceRoot: "D:\\repo",
    });
    assert.equal(
      buildAgentMemoryDirectoryHint({ name: "code-reviewer", memory: scope }, "D:/repo"),
      `${sharedRoot.replace(/\\/gu, "/")}/`,
    );
  }
});

test("漂移钉住（D26 号目录）：UI 提示与 shared 推导都按号落位，名字不参与", () => {
  const agentId = "0f6a2c1e-77db-4a1b-9c3d-5e2f8b1a4c9d";
  for (const scope of ["project", "local"] as const) {
    const sharedRoot = resolveAgentMemoryRoot({
      agentName: "code-reviewer",
      agentId,
      scope,
      storageRoot: "",
      workspaceRoot: "D:\\repo",
    });
    assert.ok(
      sharedRoot.replace(/\\/gu, "/").includes(`/.zcode/${scope === "local" ? "agent-memory-local" : "agent-memory"}/a-${agentId}`),
      "号目录带 a- 段（与老的名字目录物理区分，搬家时认得出）",
    );
    // 档案改了名也不影响：号在场即以号为准。
    assert.equal(
      buildAgentMemoryDirectoryHint({ name: "ui-pro", memory: scope, agentId }, "D:/repo"),
      `${sharedRoot.replace(/\\/gu, "/")}/`,
    );
  }
});

test("漂移钉住：persona 记忆缺省与面板展示口径（memory ?? \"project\"）同一", () => {
  // 面板/删除提示按 `memory ?? "project"` 展示记事本路径，persona 载荷的缺省必须同一口径——
  // 否则会出现「面板有记事本、会话永远不读」的语义谎言（设置页三框建档即这条路径）。
  const bareAgent = { id: "a1", name: "code-reviewer", systemPrompt: "p", scope: "workspace" } as AgentSummary;
  assert.equal(toProjectAgentPersona(bareAgent).memoryScope, "project");
  assert.equal(
    buildAgentMemoryDirectoryHint(bareAgent, "D:/repo"),
    "D:/repo/.zcode/agent-memory/code-reviewer/",
  );
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

test("applyDerivedPersonaChatBadges：persona 标题前缀 × 现役档案 → 冷重启工牌（名字+颜色）", () => {
  const agents = [
    { name: "ui-pro", color: "purple" as const },
    { name: "planner" },
  ];
  const personaRow = { taskId: "s1", title: "ui-pro · 你好呀，你是谁" };
  const colorlessRow = { taskId: "s2", title: "planner · 排个计划" };
  const plainRow = { taskId: "s3", title: "修复登录超时" };

  const [badged, colorless, plain] = applyDerivedPersonaChatBadges(
    [personaRow, colorlessRow, plainRow],
    agents,
  );
  assert.deepEqual(getPersonaChatBadge(badged as never), { name: "ui-pro", color: "purple" });
  assert.deepEqual(getPersonaChatBadge(colorless as never), { name: "planner" }, "无色档案只带名字（兜底只画图标）");
  assert.equal(getPersonaChatBadge(plain as never), undefined);
  // 未命中行保持原引用（行级 memo 靠引用判等）。
  assert.equal(plain, plainRow);
});

test("applyDerivedPersonaChatBadges：前缀陷阱不误标——ui-pro-team 的标题不吃 ui-pro 的徽章", () => {
  const agents = [{ name: "ui-pro" }];
  const teamRow = { taskId: "t1", title: "ui-pro-team · 我是独立员工" };
  const [derived] = applyDerivedPersonaChatBadges([teamRow], agents);
  assert.equal(getPersonaChatBadge(derived as never), undefined);
});

test("applyDerivedPersonaChatBadges：已带登记徽章的行原引用保留（登记层优先，反推不覆盖）", () => {
  const agents = [{ name: "ui-pro", color: "purple" as const }];
  const registered = {
    taskId: "s1",
    title: "ui-pro · 你好",
    agentPersona: { name: "ui-pro" },
  };
  const [kept] = applyDerivedPersonaChatBadges([registered], agents);
  assert.equal(kept, registered, "登记行必须原引用返回，反推不做二次覆盖");
});

test("applyDerivedPersonaChatBadges：空名单退化为等价副本，改名后按新档案名对号", () => {
  const rows = [{ taskId: "s1", title: "ui-pro · 你好" }];
  const copy = applyDerivedPersonaChatBadges(rows, []);
  assert.deepEqual(copy, rows);
  assert.notEqual(copy, rows);
  assert.equal(copy[0], rows[0], "空名单不换行引用，只换数组壳");

  // 改名跟走的边界：反推按「当前档案名」匹配标题前缀——档案改名后旧标题不再命中，
  // 这是 meta_json 落盘（第二落点）落地前的已知上限，测试钉住语义防止误以为已跟走。
  const renamed = applyDerivedPersonaChatBadges(rows, [{ name: "大龙" }]);
  assert.equal(getPersonaChatBadge(renamed[0] as never), undefined);
});

test("findPersonaRowIdsByTitlePrefix：只收旧名前缀行，改名补链的扫描边界", () => {
  const rows = [
    { taskId: "s1", title: "ui-pro · 你好" },
    { taskId: "s2", title: "ui-pro-team · 我是独立员工" },
    { taskId: "s3", title: "修复登录超时" },
    { taskId: "s4", title: "ui-pro-planner · 排计划" },
  ];
  assert.deepEqual(findPersonaRowIdsByTitlePrefix(rows, "ui-pro"), ["s1"]);
  // 分隔符带空格：ui-pro-team / ui-pro-planner 都不吃 ui-pro 的链。
  assert.deepEqual(findPersonaRowIdsByTitlePrefix(rows, "ui-pro-team"), ["s2"]);
  assert.deepEqual(findPersonaRowIdsByTitlePrefix(rows, "不存在"), []);
});

test("buildRetitledPersonaTitle：换新名前缀保历史首条输入；自定义标题不动", () => {
  assert.equal(
    buildRetitledPersonaTitle("ui-test · 你好，认识我吗", "UI-plus"),
    "UI-plus · 你好，认识我吗",
  );
  // 跨改名时代的陈旧前缀也按首个分隔符对号（员工名 [a-zA-Z0-9-] 不含「 · 」）。
  assert.equal(buildRetitledPersonaTitle("UI-pro · 排个计划", "UI-plus"), "UI-plus · 排个计划");
  // 无分隔符 = 用户自定义标题：返回 null，调用方跳过改写只补登记。
  assert.equal(buildRetitledPersonaTitle("我的装修讨论", "UI-plus"), null);
  // 名字段为空（分隔符在开头）：视为非 persona 标题，不改写。
  assert.equal(buildRetitledPersonaTitle(" · 开头就是分隔符", "UI-plus"), null);
});

test("buildRenamedPersonaTitle：有前缀换名；无前缀补回「新名 · 」（治愈卡死行）", () => {
  assert.equal(buildRenamedPersonaTitle("ui-test · 你好", "UI-plus"), "UI-plus · 你好");
  // 图四场景：用户把前缀删成只剩正文，改名必须把前缀补回来而不是永远卡住。
  assert.equal(buildRenamedPersonaTitle("你好", "UI-plus"), "UI-plus · 你好");
  assert.equal(buildRenamedPersonaTitle("  你好  ", "UI-plus"), "UI-plus ·   你好  ");
  assert.equal(buildRenamedPersonaTitle("   ", "UI-plus"), null);
});

test("resolvePersonaChatBadgeForTask：行内 → 持久登记 → 标题反推 三路优先级", () => {
  const task = { taskId: "s1", title: "code-test · 你好" };
  const registered = new Map([["s1", { name: "登记名" }]]);
  // 行内标记最优先（侧栏行已带对号结果）。
  assert.deepEqual(
    resolvePersonaChatBadgeForTask({ ...task, agentPersona: { name: "行内名" } }, registered, [
      { name: "档案名" },
    ]),
    { name: "行内名" },
  );
  // 登记其次（Header 拿裸 meta 时的主路径）。
  assert.deepEqual(resolvePersonaChatBadgeForTask(task, registered, [{ name: "档案名" }]), {
    name: "登记名",
  });
  // 反推兜底（冷重启后登记丢失、标题前缀仍在）。
  assert.deepEqual(
    resolvePersonaChatBadgeForTask(task, undefined, [{ name: "code-test", color: "purple" }]),
    { name: "code-test", color: "purple" },
  );
  // 三路都无 → undefined（普通会话行不吃员工菜单）。
  assert.equal(resolvePersonaChatBadgeForTask(task, undefined, []), undefined);
});

test("stripPersonaTitlePrefix：工牌在旁时标题不重复员工名；中文自由标题不动", () => {
  assert.equal(stripPersonaTitlePrefix("code-test · 你好", "code-test"), "你好");
  // 陈旧旧名前缀也剥（工牌已是新名）——段形似员工名即可。
  assert.equal(stripPersonaTitlePrefix("UI-pro · 你好", "code-test"), "你好");
  // 无前缀 / 中文自由标题（不形似员工名）原样保留。
  assert.equal(stripPersonaTitlePrefix("你好", "code-test"), "你好");
  assert.equal(stripPersonaTitlePrefix("装修 · 二期", "code-test"), "装修 · 二期");
  assert.equal(stripPersonaTitlePrefix(" · 分隔符在开头", "code-test"), " · 分隔符在开头");
});

test("findLatestPersonaChatRow：三路对号找员工最近一段会话，取 updatedAt 最新", () => {
  const rows = [
    { taskId: "s1", title: "ui-plus · 旧话题", updatedAt: 100 },
    { taskId: "s2", title: "ui-plus · 新话题", updatedAt: 300 },
    { taskId: "s3", title: "掉前缀的卡死行", updatedAt: 999 },
  ];
  // 掉前缀的行靠持久登记归号，且它最新 → 就应续接它。
  const registry = new Map([["s3", { name: "ui-plus" }]]);
  assert.equal(
    findLatestPersonaChatRow(rows, { name: "ui-plus" }, registry, [{ name: "ui-plus" }])?.taskId,
    "s3",
  );
  // 无登记时靠标题前缀反推，取最新前缀行。
  assert.equal(
    findLatestPersonaChatRow(rows.slice(0, 2), { name: "ui-plus" }, undefined, [
      { name: "ui-plus" },
    ])?.taskId,
    "s2",
  );
  // 没有任何归属 → undefined（首见员工走新建）。
  assert.equal(findLatestPersonaChatRow(rows, { name: "查无此人" }, new Map(), []), undefined);
});

test("findLatestArchivedPersonaChatRow：归档堆里靠标题反推捞员工最近一段（无登记可用）", () => {
  // 归档行不进侧栏列表，登记徽章重启即丢——唯一线索是存储标题的「名字 · 」前缀。
  const archived = [
    { taskId: "a1", title: "ui-plus · 一次派单任务", updatedAt: 200 },
    { taskId: "a2", title: "ui-plus · 你好", updatedAt: 500 },
    { taskId: "a3", title: "普通会话没前缀", updatedAt: 900 },
  ];
  assert.equal(findLatestArchivedPersonaChatRow(archived, { name: "ui-plus" })?.taskId, "a2");
  // 档案柜里真没有这个员工 → undefined（首见，走新开一段）。
  assert.equal(findLatestArchivedPersonaChatRow(archived, { name: "查无此人" }), undefined);
  // 前缀陷阱：员工 ui-pro 不能认领 ui-pro-team 的归档行。
  const trapped = [{ taskId: "t1", title: "ui-pro-team · 他的话题", updatedAt: 800 }];
  assert.equal(findLatestArchivedPersonaChatRow(trapped, { name: "ui-pro" }), undefined);
});

test("findLatestPersonaChatRow：按号认人，员工改名后旧会话照样续得上（D26）", () => {
  // 盘上标题还挂着旧名（改名跟走没扫到的行），档案现在叫 ui-pro。
  const rows = [
    { taskId: "s1", title: "ui-test · 旧话题", updatedAt: 100 },
    { taskId: "s2", title: "普通会话", updatedAt: 900 },
  ];
  const registry = new Map([["s1", { name: "ui-test", agentId: "id-1" }]]);
  assert.equal(
    findLatestPersonaChatRow(rows, { name: "ui-pro", agentId: "id-1" }, registry, [
      { name: "ui-pro", agentId: "id-1" },
    ])?.taskId,
    "s1",
    "登记里的名字陈旧，但号是同一个人",
  );
  // 无号档案回落到名字判据（不许比改号前更差）。
  assert.equal(
    findLatestPersonaChatRow(rows, { name: "ui-test" }, registry, [{ name: "ui-test" }])?.taskId,
    "s1",
  );
});

test("groupPersonaBadgedTaskItems：员工各一栏（首次出现序），未归号进普通组", () => {
  const rows = [
    { taskId: "a1", title: "甲", agentPersona: { name: "阿龙" } },
    { taskId: "p1", title: "普通会话" },
    { taskId: "b1", title: "乙", agentPersona: { name: "阿虎", color: "purple" as const } },
    { taskId: "a2", title: "丙", agentPersona: { name: "阿龙" } },
  ];
  const groups = groupPersonaBadgedTaskItems(rows);
  assert.deepEqual(groups.map((group) => group.key), ["agent:阿龙", "agent:阿虎", "plain"]);
  assert.deepEqual(
    groups[0].items.map((item) => item.taskId),
    ["a1", "a2"],
    "同员工会话收进同一栏且保传入顺序",
  );
  assert.deepEqual(groups[1].badge, { name: "阿虎", color: "purple" });
  assert.deepEqual(
    groups[2].items.map((item) => item.taskId),
    ["p1"],
    "未归号会话进普通组",
  );
  assert.equal(groups[2].badge, undefined);
});

// ============================================================
// 重命名弹窗的去前缀/回拼（D4 显示层配套）
// 红线：存储标题的「员工名 · 」是冷重启工牌反推的唯一线索，弹窗藏起来可以，
// 存回去少了前缀就重演「工牌丢了」。往返必须恒等。
// ============================================================

test("restorePersonaTitlePrefix：员工会话正文回拼原标题的名字前缀", () => {
  assert.equal(restorePersonaTitlePrefix("code-test · 你好", "改成这样"), "code-test · 改成这样");
  assert.equal(
    restorePersonaTitlePrefix("code-test · 你好", "code-test · 你好"),
    "code-test · 你好",
    "正文已带同前缀不重复拼（弹窗初值没动就确认的往返）",
  );
  assert.equal(
    restorePersonaTitlePrefix("code-test · 甲 · 乙", "正文"),
    "code-test · 正文",
    "首个分隔符即名字段边界；正文里用户自己打的「 · 」不再当边界",
  );
});

test("restorePersonaTitlePrefix：非员工标题原样交回", () => {
  assert.equal(restorePersonaTitlePrefix("普通任务", "改名"), "改名");
  assert.equal(restorePersonaTitlePrefix("装修 · 二期", "改名"), "改名", "中文名字段不是员工名");
  assert.equal(restorePersonaTitlePrefix("ab · x", "y"), "y", "短于档案名最小长度（3）不算前缀");
  assert.equal(restorePersonaTitlePrefix("code-test · 你好", "  "), "  ", "空正文不造悬挂前缀");
});

test("personaChatRenameDraft 与 restorePersonaTitlePrefix 往返恒等", () => {
  const badge = { name: "code-test" };
  for (const stored of ["code-test · 你好", "old-name · 历史标题", "你好", "装修 · 二期"]) {
    assert.equal(
      restorePersonaTitlePrefix(stored, personaChatRenameDraft(stored, badge)),
      stored,
      `存储标题不被显示层改写：${stored}`,
    );
  }
});

test("personaChatRenameDraft：无徽章行不改初值", () => {
  assert.equal(personaChatRenameDraft("code-test · 你好", undefined), "code-test · 你好");
  assert.equal(personaChatRenameDraft("code-test · 你好", { name: "code-test" }), "你好");
  assert.equal(
    personaChatRenameDraft("old-name · 你好", { name: "new-name" }),
    "你好",
    "陈旧旧名前缀同样剥掉，与行上显示层（stripPersonaTitlePrefix）同一判据",
  );
});

// ============================================================
// D26 片二「会话带号认人」：徽章带号 → 按现役档案刷新 → 改名不再依赖补链
// ============================================================

test("toPersonaChatBadge：号与颜色缺席就不带键（老登记形状不变）", () => {
  assert.deepEqual(toPersonaChatBadge({ name: "ui-pro" }), { name: "ui-pro" });
  assert.deepEqual(toPersonaChatBadge({ name: "ui-pro", agentId: "id-1", color: "cyan" }), {
    name: "ui-pro",
    agentId: "id-1",
    color: "cyan",
  });
  assert.deepEqual(toPersonaChatBadge({ name: "ui-pro", agentId: "" }), { name: "ui-pro" });
});

test("toProjectAgentPersona：档案的号随 persona 载荷进会话（D26 认人的起点）", () => {
  const persona = toProjectAgentPersona({
    name: "ui-pro",
    systemPrompt: "p",
    memory: "project",
    agentId: "id-1",
  } as AgentSummary);
  assert.equal(persona.agentId, "id-1");
  assert.equal(
    toProjectAgentPersona({ name: "legacy", systemPrompt: "p" } as AgentSummary).agentId,
    undefined,
    "无号老档案的载荷不带 agentId 键",
  );
});

test("refreshPersonaChatBadgeFromDirectory：号命中现役档案就换新名新色", () => {
  const stale = { name: "ui-test", agentId: "id-1", color: "red" as const };
  assert.deepEqual(
    refreshPersonaChatBadgeFromDirectory(stale, [{ name: "ui-pro", agentId: "id-1" }]),
    { name: "ui-pro", agentId: "id-1" },
    "档案改名+换色后，工牌读的是档案本体（陈旧颜色不残留）",
  );
  const orphan = { name: "ui-test", agentId: "id-1", color: "red" as const };
  assert.equal(
    refreshPersonaChatBadgeFromDirectory(orphan, [{ name: "other", agentId: "id-9" }]),
    orphan,
    "号查不到人（档案删了/在别的工作区）返回同一对象，不抹工牌、不多余换引用",
  );
  assert.deepEqual(
    refreshPersonaChatBadgeFromDirectory({ name: "ui-test" }, [{ name: "ui-pro" }]),
    { name: "ui-test" },
    "无号登记不动（按名字对号另有链路）",
  );
});

test("resolvePersonaChatBadgeForTask：登记带号时改名后行上直接读出新名（不等补链）", () => {
  const task = { taskId: "s1", title: "用户手改过的标题，没有前缀" };
  const registry = new Map<string, PersonaChatBadge>([
    ["s1", { name: "ui-test", agentId: "id-1" }],
  ]);
  assert.deepEqual(
    resolvePersonaChatBadgeForTask(task, registry, [{ name: "ui-pro", agentId: "id-1" }]),
    { name: "ui-pro", agentId: "id-1" },
  );
});

test("applyPersonaChatBadges：登记行按号刷新，未登记行保持原引用", () => {
  const plain = { taskId: "p1", title: "普通会话" };
  const rows = [plain, { taskId: "s1", title: "ui-test · 旧话题" }];
  const registry = new Map<string, PersonaChatBadge>([
    ["s1", { name: "ui-test", agentId: "id-1" }],
  ]);
  const merged = applyPersonaChatBadges(rows, registry, [
    { name: "ui-pro", agentId: "id-1", color: "green" },
  ]);
  assert.equal(merged[0], plain, "未命中行不许换引用（行级 memo 依赖）");
  assert.deepEqual(merged[1].agentPersona, {
    name: "ui-pro",
    agentId: "id-1",
    color: "green",
  });
});

test("groupPersonaBadgedTaskItems：同一员工改名前后的两段会话进同一栏（按号归并）", () => {
  const rows = [
    { taskId: "a1", agentPersona: { name: "ui-test", agentId: "id-1" } },
    { taskId: "a2", agentPersona: { name: "ui-pro", agentId: "id-1" } },
  ];
  const groups = groupPersonaBadgedTaskItems(rows);
  assert.equal(groups.length, 1, "号相同就是同一个人，不许因为改过名劈成两栏");
  assert.equal(groups[0].key, "agent:id-1");
});

test("isPersonaChatRowOfRenamedAgent：号两边都在就只认号（同名别人不许被换牌）", () => {
  // 员工 id-1 从 ui-test 改名 ui-pro；id-2 是另一个员工，登记名恰好也叫 ui-test。
  const base = { title: "ui-test · 你好", agentId: "id-1", oldName: "ui-test" };
  assert.equal(
    isPersonaChatRowOfRenamedAgent({ ...base, badges: [{ name: "ui-test", agentId: "id-1" }] }),
    true,
  );
  assert.equal(
    isPersonaChatRowOfRenamedAgent({ ...base, badges: [{ name: "ui-test", agentId: "id-2" }] }),
    false,
    "带着别人号的行 = 别人的人，名字撞车也不许换牌",
  );
  assert.equal(
    isPersonaChatRowOfRenamedAgent({ ...base, badges: [{ name: "ui-test" }] }),
    true,
    "号之前的老登记按旧名",
  );
  assert.equal(
    isPersonaChatRowOfRenamedAgent({ ...base, badges: [undefined] }),
    true,
    "没有徽章的行仍靠标题前缀认（D2 老判据不丢）",
  );
  assert.equal(
    isPersonaChatRowOfRenamedAgent({ ...base, badges: [], title: "用户手改过的标题" }),
    false,
  );
  assert.equal(
    isPersonaChatRowOfRenamedAgent({ title: "ui-test · x", badges: [{ name: "ui-test" }], oldName: "ui-test" }),
    true,
    "无号档案的改名：整套老判据原样生效",
  );
});

test("derivePersonaChatBadge：标题反推命中现役档案时顺手把号带上（D26）", () => {
  const badge = derivePersonaChatBadge(
    { title: "ui-pro · 你好" },
    [{ name: "ui-pro", agentId: "id-1", color: "purple" }],
  );
  assert.deepEqual(badge, { name: "ui-pro", agentId: "id-1", color: "purple" });
});
