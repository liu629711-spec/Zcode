// ============================================================
// persistent-memory 的可运行检查（tsx --test，persistent-memory.ts
// 内部 import 用 .js 描述符，Node 原生剥离类型不重写，须经 tsx 加载）
// ============================================================
// projectPersistentAgentMemoryTools 的两侧投影：
// - 子代理派遣（profiles）：withPersistentAgentMemoryTools 追加 Write/Edit（既有语义）；
// - 驻场主会话（G2/D8）：persona 会话把档案 tools 白名单落到会话级 toolAllowlist 时，
//   记忆写入端 Write/Edit 必须随身份在场，且只受同一记忆总开关门控。
// 普通（无 persona）会话两侧都不动——行为字节级不变。
// loadProjectAgentMemoryPrompt（G7 口径统一）：scope 按 persona 携带值读（不写死
// project，user 走 storageRoot 分支），门控镜像同一记忆总开关。
//
// 运行：./node_modules/.bin/tsx --test apps/zcode-cli/packages/core/src/subagent/persistent-memory.test.ts

import assert from "node:assert/strict";
import { join, resolve } from "node:path";
import { test } from "node:test";
import type { FileSystemPort } from "@zcode/contracts";
import type { AgentRuntimeConfig } from "../runtime/types.js";
import {
  loadProjectAgentMemoryPrompt,
  projectPersistentAgentMemoryTools,
} from "./persistent-memory.ts";

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

// ---------------------------------------------------------------------------
// loadProjectAgentMemoryPrompt（G7）：注入按 persona 携带的 scope 读、门控镜像
// 记忆总开关。假 FileSystemPort 记录每次读盘/建目录，落点期望值用 shared 的
// resolveAgentMemoryRoot 同源推导——注入与面板/子代理派生漂移即测试失败。
// ---------------------------------------------------------------------------

const WORKSPACE_ROOT = resolve("/tmp/zcode-ws");

function createMemoryFileSystem(files: ReadonlyMap<string, string>): {
  port: FileSystemPort;
  readPaths: string[];
  createdDirectories: string[];
} {
  const readPaths: string[] = [];
  const createdDirectories: string[] = [];
  const port = {
    async createDirectory({ path }: { path: string }): Promise<void> {
      createdDirectories.push(path);
    },
    async readTextFile({ path }: { path: string }): Promise<{ content: string }> {
      readPaths.push(path);
      const content = files.get(path);
      if (content === undefined) throw new Error(`ENOENT: ${path}`);
      return { content };
    },
  } as unknown as FileSystemPort;
  return { port, readPaths, createdDirectories };
}

function loadPrompt(input: {
  port: FileSystemPort;
  memory?: { enabled?: boolean; use?: boolean; storageRoot?: string };
  memoryScope: "user" | "project" | "local";
  agentName?: string;
  agentId?: string;
}): Promise<string | undefined> {
  return loadProjectAgentMemoryPrompt({
    fileSystemPort: input.port,
    agentName: input.agentName ?? "code-reviewer",
    ...(input.agentId ? { agentId: input.agentId } : {}),
    memory: input.memory,
    memoryScope: input.memoryScope,
    workspaceRoot: WORKSPACE_ROOT,
  });
}

test("驻场记忆注入：总开关关闭（enabled=false/use=false/无配置）不注入也不建目录", async () => {
  for (const memory of [
    undefined,
    { enabled: false, use: true, storageRoot: "<root>" },
    { enabled: true, use: false, storageRoot: "<root>" },
    { enabled: true, use: true, storageRoot: "" },
  ]) {
    const { port, readPaths, createdDirectories } = createMemoryFileSystem(new Map());
    const prompt = await loadPrompt({ port, memory, memoryScope: "project" });
    assert.equal(prompt, undefined, JSON.stringify(memory));
    assert.deepEqual(readPaths, [], JSON.stringify(memory));
    assert.deepEqual(createdDirectories, [], JSON.stringify(memory));
  }
});

test("驻场记忆注入（双层）：project scope 读工作区本子 + 随身本子两处 MEMORY.md，且不现造目录", async () => {
  const workspaceDir = join(WORKSPACE_ROOT, ".zcode", "agent-memory", "code-reviewer");
  const personalDir = join(MEMORY_ON.storageRoot, "agent-memory", "code-reviewer");
  const files = new Map([
    [join(workspaceDir, "MEMORY.md"), "- [old work](old.md) — context"],
    [join(personalDir, "MEMORY.md"), "- [taste](taste.md) — likes"],
  ]);
  const { port, readPaths, createdDirectories } = createMemoryFileSystem(files);
  const prompt = await loadPrompt({ port, memory: MEMORY_ON, memoryScope: "project" });
  assert.ok(prompt);
  assert.deepEqual(readPaths, [
    join(workspaceDir, "MEMORY.md"),
    join(personalDir, "MEMORY.md"),
  ]);
  // D25 幻影目录防线：读路径绝不建目录——改过名的旧会话按快照旧名恢复时，
  // 曾在这里把空目录造在旧 key 下，挡住档案改名的记事本搬家（记忆"没了"）。
  // 目录由首次真实写入（Write createParents）落盘，读取缺席 = 空索引基线。
  assert.deepEqual(createdDirectories, []);
  assert.ok(prompt.includes(join(workspaceDir, "\\")), "prompt 应带工作区本子根目录");
  assert.ok(prompt.includes(join(personalDir, "\\")), "prompt 应带随身本子根目录");
  assert.ok(prompt.includes("- [old work](old.md) — context"));
  assert.ok(prompt.includes("- [taste](taste.md) — likes"));
});

test("驻场记忆注入（2026-10-02 拍板）：user scope 不再把工作区本子整个搬去家目录——工作区层恒在 <ws>，家目录只剩随身层", async () => {
  const workspaceDir = join(WORKSPACE_ROOT, ".zcode", "agent-memory", "code-reviewer");
  const personalDir = join(MEMORY_ON.storageRoot, "agent-memory", "code-reviewer");
  const files = new Map([[join(personalDir, "MEMORY.md"), "- [taste](taste.md) — likes"]]);
  const { port, readPaths } = createMemoryFileSystem(files);
  const prompt = await loadPrompt({ port, memory: MEMORY_ON, memoryScope: "user" });
  assert.ok(prompt);
  assert.deepEqual(readPaths, [
    join(workspaceDir, "MEMORY.md"),
    join(personalDir, "MEMORY.md"),
  ]);
  assert.ok(prompt.includes(join(workspaceDir, "\\")));
});

test("驻场记忆注入（双层分工）：随身本子明写「项目事实绝不入随身」，桌子铁规矩点名家目录", async () => {
  const { port } = createMemoryFileSystem(new Map());
  const prompt = await loadPrompt({ port, memory: MEMORY_ON, memoryScope: "project" });
  assert.ok(prompt);
  assert.ok(
    prompt.includes("NEVER save project-specific facts, decisions, or work-in-progress here"),
    "随身本子要写死分账规矩",
  );
  assert.ok(
    prompt.includes("The user's home directory and any other absolute path outside the workspace are NOT your desks"),
    "桌子铁规矩要点名家目录（真机 2026-10-02：弱模型把 PRD 写进家目录）",
  );
  assert.ok(
    prompt.includes("never write deliverables, drafts, or scratch files there"),
    "桌子铁规矩要写明产出不外放",
  );
});

test("驻场记忆注入：local scope 的工作区层读 <ws>/.zcode/agent-memory-local/<key>/，随身层照旧", async () => {
  const workspaceDir = join(WORKSPACE_ROOT, ".zcode", "agent-memory-local", "code-reviewer");
  const personalDir = join(MEMORY_ON.storageRoot, "agent-memory", "code-reviewer");
  const { port, readPaths } = createMemoryFileSystem(
    new Map([[join(workspaceDir, "MEMORY.md"), "- [machine](m.md) — local only"]]),
  );
  const prompt = await loadPrompt({ port, memory: MEMORY_ON, memoryScope: "local" });
  assert.ok(prompt);
  assert.deepEqual(readPaths, [
    join(workspaceDir, "MEMORY.md"),
    join(personalDir, "MEMORY.md"),
  ]);
  assert.ok(prompt.includes("- [machine](m.md) — local only"));
});

test("驻场记忆注入（D26）：号在场两本本子都读 a-<uuid> 目录，档案叫什么都不影响落点", async () => {
  const agentId = "0f6a2c1e-77db-4a1b-9c3d-5e2f8b1a4c9d";
  // 名字目录里留着旧内容（改名前的历史落点），号目录里才是现在的记事本。
  const staleDir = join(WORKSPACE_ROOT, ".zcode", "agent-memory", "ui-test");
  const workspaceDir = join(WORKSPACE_ROOT, ".zcode", "agent-memory", `a-${agentId}`);
  const personalDir = join(MEMORY_ON.storageRoot, "agent-memory", `a-${agentId}`);
  const { port, readPaths } = createMemoryFileSystem(
    new Map([
      [join(staleDir, "MEMORY.md"), "- [陈旧](stale.md) — 旧名字目录"],
      [join(workspaceDir, "MEMORY.md"), "- [现职](now.md) — 号目录里的记忆"],
    ]),
  );
  const prompt = await loadPrompt({
    port,
    memory: MEMORY_ON,
    memoryScope: "project",
    agentName: "ui-pro",
    agentId,
  });
  assert.ok(prompt);
  assert.deepEqual(
    readPaths,
    [join(workspaceDir, "MEMORY.md"), join(personalDir, "MEMORY.md")],
    "只读号目录（工作区层+随身层），不碰名字目录",
  );
  assert.ok(prompt.includes("- [现职](now.md) — 号目录里的记忆"));
});

test("驻场记忆注入（D26）：坏号当无号，回落名字目录（绝不用乱码拼路径）", async () => {
  const workspaceDir = join(WORKSPACE_ROOT, ".zcode", "agent-memory", "code-reviewer");
  const personalDir = join(MEMORY_ON.storageRoot, "agent-memory", "code-reviewer");
  const { port, readPaths } = createMemoryFileSystem(
    new Map([[join(workspaceDir, "MEMORY.md"), "- [ok](ok.md) — 基线"]]),
  );
  const prompt = await loadPrompt({
    port,
    memory: MEMORY_ON,
    memoryScope: "project",
    agentId: "../../escape",
  });
  assert.ok(prompt);
  assert.deepEqual(readPaths, [
    join(workspaceDir, "MEMORY.md"),
    join(personalDir, "MEMORY.md"),
  ]);
});

test("驻场记忆注入：两本 MEMORY.md 都缺失不炸，返回双空索引基线 prompt，且零建目录（D25）", async () => {
  const { port, readPaths, createdDirectories } = createMemoryFileSystem(new Map());
  const prompt = await loadPrompt({ port, memory: MEMORY_ON, memoryScope: "project" });
  assert.ok(prompt);
  assert.equal(readPaths.length, 2);
  assert.ok(prompt.includes("Your workspace notebook's MEMORY.md is currently empty"));
  assert.ok(prompt.includes("Your personal notebook's MEMORY.md is currently empty"));
  // 目录缺席时的读取也不许把目录造出来（幻影目录防线，D25）。
  assert.deepEqual(createdDirectories, []);
});
