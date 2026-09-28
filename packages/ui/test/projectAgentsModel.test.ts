import assert from "node:assert/strict";
import test from "node:test";
import type { AgentSummary } from "@zcode/shared";
import { resolveAgentMemoryRoot } from "@zcode/shared/node";
import {
  applyDerivedPersonaChatBadges,
  applyPersonaChatBadges,
  buildAgentMemoryDirectoryHint,
  buildRetitledPersonaTitle,
  findPersonaRowIdsByTitlePrefix,
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
