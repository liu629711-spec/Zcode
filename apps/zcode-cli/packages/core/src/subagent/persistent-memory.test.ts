// ============================================================
// persistent-memory 工具面投影的可运行检查（tsx --test，persistent-memory.ts
// 内部 import 用 .js 描述符，Node 原生剥离类型不重写，须经 tsx 加载）
// ============================================================
// projectPersistentAgentMemoryTools 的两侧投影：
// - 子代理派遣（profiles）：withPersistentAgentMemoryTools 追加 Write/Edit（既有语义）；
// - 驻场主会话（G2/D8）：persona 会话把档案 tools 白名单落到会话级 toolAllowlist 时，
//   记忆写入端 Write/Edit 必须随身份在场，且只受同一记忆总开关门控。
// 普通（无 persona）会话两侧都不动——行为字节级不变。
//
// 运行：./node_modules/.bin/tsx --test apps/zcode-cli/packages/core/src/subagent/persistent-memory.test.ts

import assert from "node:assert/strict";
import { test } from "node:test";
import type { AgentRuntimeConfig } from "../runtime/types.js";
import { projectPersistentAgentMemoryTools } from "./persistent-memory.ts";

const MEMORY_ON = { enabled: true, use: true, storageRoot: "<dataRoot>/memory" } as const;

function config(input: Partial<AgentRuntimeConfig>): AgentRuntimeConfig {
  return input as AgentRuntimeConfig;
}

test("主会话白名单补记忆写入工具：persona + 档案白名单在场 + 总开关开启", () => {
  const projected = projectPersistentAgentMemoryTools(
    config({
      memory: MEMORY_ON,
      projectAgentPersona: { name: "code-reviewer", memory: "project" },
      toolAllowlist: ["Read", "Grep"],
    }),
  );
  assert.deepEqual(projected.toolAllowlist, ["Read", "Grep", "Write", "Edit"]);
});

test("已含 Write/Edit 时不重复追加", () => {
  const projected = projectPersistentAgentMemoryTools(
    config({
      memory: MEMORY_ON,
      projectAgentPersona: { name: "code-reviewer", memory: "project" },
      toolAllowlist: ["Write", "Grep"],
    }),
  );
  assert.deepEqual(projected.toolAllowlist, ["Write", "Grep", "Edit"]);
});

test("总开关三重门：enabled=false / use=false / 无 storageRoot 都不追加", () => {
  const persona = { name: "code-reviewer", memory: "project" as const };
  const cases = [
    { enabled: false, use: true, storageRoot: "<root>" },
    { enabled: true, use: false, storageRoot: "<root>" },
    { enabled: true, use: true, storageRoot: "" },
  ];
  for (const memory of cases) {
    const projected = projectPersistentAgentMemoryTools(
      config({ memory, projectAgentPersona: persona, toolAllowlist: ["Read"] }),
    );
    assert.deepEqual(projected.toolAllowlist, ["Read"], JSON.stringify(memory));
  }
});

test("白名单缺席（继承全部工具）或 persona 无记忆 scope：不动工具面", () => {
  const personaWithMemory = { name: "code-reviewer", memory: "project" as const };
  assert.equal(
    projectPersistentAgentMemoryTools(
      config({ memory: MEMORY_ON, projectAgentPersona: personaWithMemory }),
    ).toolAllowlist,
    undefined,
  );
  assert.deepEqual(
    projectPersistentAgentMemoryTools(
      config({
        memory: MEMORY_ON,
        projectAgentPersona: { name: "code-reviewer" },
        toolAllowlist: ["Read"],
      }),
    ).toolAllowlist,
    ["Read"],
  );
});

test("普通会话（无 persona）行为不变：无 profiles 时原样返回", () => {
  const plain = config({ memory: MEMORY_ON, toolAllowlist: ["Read"] });
  assert.equal(projectPersistentAgentMemoryTools(plain), plain);
});

test("子代理派遣投影保持既有语义，且与主会话补齐互不干扰", () => {
  const profiles = [
    { name: "a", systemPrompt: "a", memory: "project" as const, tools: ["Read"] },
    { name: "b", systemPrompt: "b" },
  ];
  const projected = projectPersistentAgentMemoryTools(
    config({
      memory: MEMORY_ON,
      projectAgentPersona: { name: "host", memory: "project" },
      toolAllowlist: ["Grep"],
      subagents: { profiles },
    }),
  );
  assert.deepEqual(projected.toolAllowlist, ["Grep", "Write", "Edit"]);
  assert.deepEqual(projected.subagents?.profiles?.[0]?.tools, ["Read", "Write", "Edit"]);
  // 无 memory 的 profile 不追加（withPersistentAgentMemoryTools 既有语义）。
  assert.equal(projected.subagents?.profiles?.[1]?.tools, undefined);
});
