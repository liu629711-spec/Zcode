// ============================================================
// persona-session 的可运行检查（node --test，Node 24 原生剥离类型直跑）
// ============================================================
// 驻场智能体主会话的四条纯拼装规则：system prompt 多段拼接、标题记账、
// persona 落盘快照（G1：name/systemPrompt 任一缺席 → 不落盘）、
// persona → runtime config 映射（G2：模型只随 create、工具面随身份、color 随行）。
//
// 运行：node --test apps/zcode-cli/packages/core/src/subagent/persona-session.test.ts
// （core 的 tsconfig 已排除 **/*.test.ts，tsc 构建不受影响。）

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  buildProjectAgentPersonaSnapshot,
  buildProjectAgentSessionTitle,
  joinPersonaSystemPrompt,
  mapPersonaToRuntimeConfig,
} from "./persona-session.ts";

test("joinPersonaSystemPrompt：persona 与记忆段按 \\n\\n 拼接", () => {
  assert.equal(
    joinPersonaSystemPrompt(["你是代码审查员", "## Persistent Agent Memory\n..."]),
    "你是代码审查员\n\n## Persistent Agent Memory\n...",
  );
});

test("joinPersonaSystemPrompt：空段跳过，全空返回 undefined", () => {
  assert.equal(joinPersonaSystemPrompt([undefined, "", "persona"]), "persona");
  assert.equal(joinPersonaSystemPrompt([undefined, ""]), undefined);
  assert.equal(joinPersonaSystemPrompt([]), undefined);
});

test("buildProjectAgentSessionTitle：智能体名 · 首条输入", () => {
  assert.equal(
    buildProjectAgentSessionTitle("code-reviewer", "修复登录超时"),
    "code-reviewer · 修复登录超时",
  );
});

test("buildProjectAgentPersonaSnapshot：完整快照含 memoryScope", () => {
  assert.deepEqual(
    buildProjectAgentPersonaSnapshot(
      { name: "code-reviewer", memory: "project" },
      "你是代码审查员",
    ),
    { name: "code-reviewer", systemPrompt: "你是代码审查员", memoryScope: "project" },
  );
});

test("buildProjectAgentPersonaSnapshot：无 memory 时不带 memoryScope 键", () => {
  assert.deepEqual(
    buildProjectAgentPersonaSnapshot({ name: "小助手" }, "你好"),
    { name: "小助手", systemPrompt: "你好" },
  );
});

test("buildProjectAgentPersonaSnapshot：缺 systemPrompt/缺 persona/空名 → undefined（不落半截快照）", () => {
  assert.equal(buildProjectAgentPersonaSnapshot({ name: "小助手" }, undefined), undefined);
  assert.equal(buildProjectAgentPersonaSnapshot({ name: "小助手" }, ""), undefined);
  assert.equal(buildProjectAgentPersonaSnapshot(undefined, "你好"), undefined);
  assert.equal(buildProjectAgentPersonaSnapshot({ name: "" }, "你好"), undefined);
});

test("buildProjectAgentPersonaSnapshot：color 随快照落盘（G2 徽章取色）", () => {
  assert.deepEqual(
    buildProjectAgentPersonaSnapshot(
      { name: "code-reviewer", memory: "project", color: "purple" },
      "你是代码审查员",
    ),
    {
      name: "code-reviewer",
      systemPrompt: "你是代码审查员",
      memoryScope: "project",
      color: "purple",
    },
  );
  assert.deepEqual(buildProjectAgentPersonaSnapshot({ name: "小助手" }, "你好"), {
    name: "小助手",
    systemPrompt: "你好",
  });
});

test("buildProjectAgentPersonaSnapshot：档案工具面随快照落盘（resume/fork 回灌数据源），模型不落", () => {
  assert.deepEqual(
    buildProjectAgentPersonaSnapshot(
      {
        name: "code-reviewer",
        memory: "project",
        color: "purple",
        tools: ["Read", "Grep"],
        disallowedTools: ["Bash"],
      },
      "你是代码审查员",
    ),
    {
      name: "code-reviewer",
      systemPrompt: "你是代码审查员",
      memoryScope: "project",
      color: "purple",
      tools: ["Read", "Grep"],
      disallowedTools: ["Bash"],
    },
  );
  // 空数组与缺席同义：不落键。
  assert.deepEqual(
    buildProjectAgentPersonaSnapshot({ name: "小助手", tools: [] }, "你好"),
    { name: "小助手", systemPrompt: "你好" },
  );
});

test("mapPersonaToRuntimeConfig：create 路径全字段映射（模型/工具/颜色）", () => {
  assert.deepEqual(
    mapPersonaToRuntimeConfig(
      {
        name: "code-reviewer",
        systemPrompt: "你是代码审查员",
        memoryScope: "project",
        modelSelection: { providerId: "openai", modelId: "gpt-5" },
        tools: ["Read", "Grep"],
        disallowedTools: ["Bash"],
        color: "purple",
      },
      { includeModelSelection: true },
    ),
    {
      modelSelection: { providerId: "openai", modelId: "gpt-5" },
      toolAllowlist: ["Read", "Grep"],
      toolDisallowlist: ["Bash"],
      systemPrompt: "你是代码审查员",
      projectAgentPersona: {
        name: "code-reviewer",
        memory: "project",
        color: "purple",
        tools: ["Read", "Grep"],
        disallowedTools: ["Bash"],
      },
    },
  );
});

test("mapPersonaToRuntimeConfig：resume/fork 回灌不映射模型（模型只由 entry 恢复），工具面随身份", () => {
  const fragment = mapPersonaToRuntimeConfig(
    {
      name: "code-reviewer",
      systemPrompt: "你是代码审查员",
      modelSelection: { providerId: "openai", modelId: "gpt-5" },
      tools: ["Read"],
    },
    { includeModelSelection: false },
  );
  assert.equal(fragment.modelSelection, undefined);
  assert.deepEqual(fragment.toolAllowlist, ["Read"]);
  assert.deepEqual(fragment.projectAgentPersona, {
    name: "code-reviewer",
    tools: ["Read"],
  });
});

test("mapPersonaToRuntimeConfig：空工具数组与缺席同义（继承全部工具），最小 persona 无可选键", () => {
  const fragment = mapPersonaToRuntimeConfig(
    { name: "小助手", systemPrompt: "你好", tools: [], disallowedTools: [] },
    { includeModelSelection: true },
  );
  assert.equal(fragment.toolAllowlist, undefined);
  assert.equal(fragment.toolDisallowlist, undefined);
  assert.equal(fragment.modelSelection, undefined);
  assert.deepEqual(fragment.projectAgentPersona, { name: "小助手" });
});
