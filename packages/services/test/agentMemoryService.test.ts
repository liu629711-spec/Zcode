import assert from "node:assert/strict";
import test from "node:test";
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { createMemoryService } from "../src/memory/memoryService.js";
import { setDataBaseDir } from "../src/paths.js";

// ============================================================
// agent-memory 面板服务面（G3）的可运行检查：列表快照、读改删 round-trip、
// 路径/文件名白名单拒绝。目录推导与 core 同源由 agentMemoryPaths.test.ts 与
// packages/ui/test 的双向测试钉住。
//
// 运行：npx tsx --test packages/services/test/agentMemoryService.test.ts
// ============================================================

async function makeTempRoot(prefix: string): Promise<string> {
  return mkdtemp(join(tmpdir(), `zcode-agent-memory-${prefix}-`));
}

function seedWorkspace(base: string): { workspacePath: string; memoryRoot: string } {
  const workspacePath = join(base, "ws");
  const memoryRoot = join(workspacePath, ".zcode", "agent-memory", "Code-Reviewer");
  return { workspacePath, memoryRoot };
}

const PROJECT_PARAMS = {
  agentName: "Code Reviewer",
  scope: "project" as const,
};

test("list：目录不存在回空集不报错；rootDir 说真话（key 已清洗）", async () => {
  const base = await makeTempRoot("empty");
  try {
    const { workspacePath, memoryRoot } = seedWorkspace(base);
    const service = createMemoryService();
    const catalog = await service.listAgentMemoryFiles({
      ...PROJECT_PARAMS,
      workspacePath,
    });
    assert.equal(catalog.files.length, 0);
    assert.equal(catalog.rootDir, memoryRoot);
    assert.equal(catalog.scope, "project");
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("list：只收 .md，索引排最前，其余按名称排序；非 md 与子目录不进面板", async () => {
  const base = await makeTempRoot("list");
  try {
    const { workspacePath, memoryRoot } = seedWorkspace(base);
    await mkdir(memoryRoot, { recursive: true });
    await writeFile(join(memoryRoot, "user_role.md"), "---\nname: user-role\n---\n", "utf-8");
    await writeFile(join(memoryRoot, "MEMORY.md"), "- [用户](user_role.md)\n", "utf-8");
    await writeFile(join(memoryRoot, "feedback_testing.md"), "###\n", "utf-8");
    await writeFile(join(memoryRoot, "notes.txt"), "not a memory", "utf-8");
    await mkdir(join(memoryRoot, "sub.md"), { recursive: true });

    const service = createMemoryService();
    const catalog = await service.listAgentMemoryFiles({
      ...PROJECT_PARAMS,
      workspacePath,
    });
    assert.deepEqual(
      catalog.files.map((file) => `${file.kind}:${file.name}`),
      ["index:MEMORY.md", "item:feedback_testing.md", "item:user_role.md"],
    );
    assert.ok(catalog.files[0].updatedAt > 0);
    assert.ok(catalog.files[0].size > 0);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("read/write/delete round-trip：改写落盘、删除后读报错", async () => {
  const base = await makeTempRoot("roundtrip");
  try {
    const { workspacePath, memoryRoot } = seedWorkspace(base);
    await mkdir(memoryRoot, { recursive: true });
    const original = "---\nname: user-role\n---\n\n记得用户是数据科学家。\n";
    await writeFile(join(memoryRoot, "user_role.md"), original, "utf-8");

    const service = createMemoryService();
    const params = { ...PROJECT_PARAMS, workspacePath };

    const read = await service.readAgentMemoryFile({ ...params, fileName: "user_role.md" });
    assert.equal(read.content, original);
    assert.ok(read.updatedAt > 0);

    const updated = "---\nname: user-role\n---\n\n改：用户改行做前端了。\n";
    const written = await service.writeAgentMemoryFile({
      ...params,
      fileName: "user_role.md",
      content: updated,
    });
    assert.ok(written.updatedAt >= read.updatedAt);
    assert.equal(await readFile(join(memoryRoot, "user_role.md"), "utf-8"), updated);

    await service.deleteAgentMemoryFile({ ...params, fileName: "user_role.md" });
    await assert.rejects(
      service.readAgentMemoryFile({ ...params, fileName: "user_role.md" }),
      /ENOENT/,
    );
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("拒绝面：面板只改列表里的 .md，不建新文件、不越界、不吃大小写别名", async () => {
  const base = await makeTempRoot("reject");
  try {
    const { workspacePath, memoryRoot } = seedWorkspace(base);
    await mkdir(memoryRoot, { recursive: true });
    await writeFile(join(memoryRoot, "MEMORY.md"), "index\n", "utf-8");

    const service = createMemoryService();
    const params = { ...PROJECT_PARAMS, workspacePath };
    const write = (fileName: string) =>
      service.writeAgentMemoryFile({ ...params, fileName, content: "x" });

    await assert.rejects(write("new_entry.md"), /does not match exactly|ENOENT/);
    await assert.rejects(write("../escape.md"), /Invalid agent memory file name/);
    await assert.rejects(write("notes.txt"), /Invalid agent memory file name/);
    await assert.rejects(write(""), /Invalid agent memory file name/);
    await assert.rejects(
      service.readAgentMemoryFile({ ...params, fileName: "sub/dir.md" }),
      /Invalid agent memory file name/,
    );
    // 大小写不敏感文件系统上别名能命中磁盘文件，也必须拒绝（照 Project Memory 先例）。
    await assert.rejects(
      service.readAgentMemoryFile({ ...params, fileName: "memory.md" }),
      /does not match exactly|ENOENT/,
    );
    // 拒绝动作没有落盘。
    assert.equal(await readFile(join(memoryRoot, "MEMORY.md"), "utf-8"), "index\n");
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("scope 落点：local 走 agent-memory-local；project 缺 workspacePath 报错", async () => {
  const base = await makeTempRoot("scope");
  try {
    const { workspacePath } = seedWorkspace(base);
    const localRoot = join(workspacePath, ".zcode", "agent-memory-local", "Code-Reviewer");
    await mkdir(localRoot, { recursive: true });
    await writeFile(join(localRoot, "local_note.md"), "local\n", "utf-8");

    const service = createMemoryService();
    const localCatalog = await service.listAgentMemoryFiles({
      agentName: "Code Reviewer",
      scope: "local",
      workspacePath,
    });
    assert.equal(localCatalog.rootDir, localRoot);
    assert.deepEqual(localCatalog.files.map((file) => file.name), ["local_note.md"]);
    const read = await service.readAgentMemoryFile({
      agentName: "Code Reviewer",
      scope: "local",
      workspacePath,
      fileName: "local_note.md",
    });
    assert.equal(read.content, "local\n");

    await assert.rejects(
      service.listAgentMemoryFiles({ agentName: "a", scope: "project" }),
      /requires a workspace path/,
    );
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("user 档案：记事本在用户数据目录 <dataBaseDir>/.zcode/agent-memory/<key>/", async () => {
  const base = await makeTempRoot("user");
  setDataBaseDir(base);
  try {
    const memoryRoot = join(base, ".zcode", "agent-memory", "Global-Tutor");
    await mkdir(memoryRoot, { recursive: true });
    await writeFile(join(memoryRoot, "MEMORY.md"), "- [偏好](taste.md)\n", "utf-8");
    await writeFile(join(memoryRoot, "taste.md"), "喜欢简洁回复\n", "utf-8");

    const service = createMemoryService();
    // user 档案不依赖 workspacePath：省略也要能定位到用户数据目录。
    const catalog = await service.listAgentMemoryFiles({
      agentName: "Global Tutor",
      scope: "user",
    });
    assert.equal(catalog.rootDir, memoryRoot);
    assert.deepEqual(catalog.files.map((file) => file.name), ["MEMORY.md", "taste.md"]);
    const read = await service.readAgentMemoryFile({
      agentName: "Global Tutor",
      scope: "user",
      fileName: "taste.md",
    });
    assert.equal(read.content, "喜欢简洁回复\n");
  } finally {
    setDataBaseDir(null);
    await rm(base, { recursive: true, force: true });
  }
});

test("安全防护：符号链接文件不进列表、按名读取被拒", { skip: process.platform === "win32" }, async () => {
  const base = await makeTempRoot("symlink");
  try {
    const { workspacePath, memoryRoot } = seedWorkspace(base);
    await mkdir(memoryRoot, { recursive: true });
    await writeFile(join(memoryRoot, "real.md"), "真身\n", "utf-8");
    await symlink(join(memoryRoot, "real.md"), join(memoryRoot, "alias.md"));
    await symlink(join(base, "outside.md"), join(memoryRoot, "outside.md"));

    const service = createMemoryService();
    const params = { ...PROJECT_PARAMS, workspacePath };
    const catalog = await service.listAgentMemoryFiles(params);
    assert.deepEqual(catalog.files.map((file) => file.name), ["real.md"]);
    await assert.rejects(
      service.readAgentMemoryFile({ ...params, fileName: "alias.md" }),
      /not a regular file/,
    );
    await assert.rejects(
      service.readAgentMemoryFile({ ...params, fileName: "outside.md" }),
      /not a regular file/,
    );
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});
