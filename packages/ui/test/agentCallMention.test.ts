import assert from "node:assert/strict";
import test from "node:test";
import {
  buildAgentCallMentionMarkdown,
  parseMentionMarkdown,
} from "../src/mentions/mentionMarkdown.js";
import { mapProjectAgentsToCallMentionItems } from "../src/mentions/providers/agentCallMentionProvider.js";

// ============================================================
// D24「@点将」的引用载体：[@员工名](agent://员工名)。
// 身份只在 destination（label 随便改），core 的 agent-call 解析器按同一条名字判据
// （[A-Za-z0-9-]{3,50}，与档案名字符集一致）认人；两边判据在这里对账。
//
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/agentCallMention.test.ts
// ============================================================

const AGENT_NAME_PATTERN = /^[A-Za-z0-9-]{3,50}$/;

test("buildAgentCallMentionMarkdown：写清身份在 destination，特殊字符转义", () => {
  assert.equal(
    buildAgentCallMentionMarkdown("doc-writer", "doc-writer"),
    "[@doc-writer](agent://doc-writer)",
  );
  const escaped = buildAgentCallMentionMarkdown("a]b", "code-builder");
  assert.ok(escaped.startsWith("[@a\\]b]"), `label 里的 ] 必须转义：${escaped}`);
  assert.ok(escaped.endsWith("(agent://code-builder)"));
});

test("parseMentionMarkdown：认得点将引用，label 不当身份", () => {
  const parts = parseMentionMarkdown("先 [@doc-writer](agent://doc-writer) 写文档");
  assert.deepEqual(parts, [
    { type: "text", text: "先 " },
    { type: "subagent", label: "doc-writer", agentName: "doc-writer" },
    { type: "text", text: " 写文档" },
  ]);
  // label 与 destination 不一致时以 destination 为身份（core 也只看 destination）。
  const mismatched = parseMentionMarkdown("[@显示名](agent://doc-writer)");
  assert.deepEqual(mismatched, [
    { type: "subagent", label: "显示名", agentName: "doc-writer" },
  ]);
});

test("parseMentionMarkdown：坏形状不赋身份，普通链接与裸 @ 写法行为不变", () => {
  assert.deepEqual(parseMentionMarkdown("[@x](agent://中文名)"), [
    { type: "subagent", label: "x" },
  ]);
  assert.deepEqual(parseMentionMarkdown("[@x](agent://)"), [{ type: "subagent", label: "x" }]);
  // 旧行为：正文里手打的 @name 仍是展示层 subagent，不带身份、不触发点将。
  assert.deepEqual(parseMentionMarkdown("@doc-writer 你好"), [
    { type: "subagent", label: "doc-writer" },
    { type: "text", text: " 你好" },
  ]);
  // 文件 mention 不许被新分支抢走。
  assert.deepEqual(parseMentionMarkdown("[@README](./README.md)"), [
    { type: "file", label: "README" },
  ]);
});

test("UI 与 core 同判据：能成 chip 的名字都能被 core 认出来", () => {
  const cases: readonly [string, boolean][] = [
    ["doc-writer", true],
    ["UI-pro", true],
    ["ab", false],
    ["a".repeat(51), false],
    ["中文名", false],
    ["doc writer", false],
  ];
  for (const [name, legal] of cases) {
    const built = buildAgentCallMentionMarkdown(name, name);
    const parsed = parseMentionMarkdown(built)[0];
    assert.equal(
      parsed.type === "subagent" && parsed.agentName !== undefined,
      legal,
      `${name} → ${built} 的 agentName 判据应与 ${AGENT_NAME_PATTERN} 一致`,
    );
  }
});

test("mapProjectAgentsToCallMentionItems：过滤非法名、按名字与介绍匹配 query", () => {
  const directory = [
    { name: "doc-writer", description: "把口述需求整理成能看的文档" },
    { name: "中文名", description: "不该进候选（派不动）" },
    { name: "code-builder", description: "照需求改代码", color: "green" },
  ] as const;
  assert.deepEqual(
    mapProjectAgentsToCallMentionItems(directory, "").map((item) => item.label),
    ["doc-writer", "code-builder"],
  );
  assert.deepEqual(
    mapProjectAgentsToCallMentionItems(directory, "文档").map((item) => item.label),
    ["doc-writer"],
    "介绍也要能搜到（小白记不住英文名）",
  );
  assert.deepEqual(mapProjectAgentsToCallMentionItems(directory, "zzz"), []);

  const item = mapProjectAgentsToCallMentionItems(directory, "code")[0]!;
  assert.equal(item.category, "subagents");
  assert.equal(item.markdown, "[@code-builder](agent://code-builder)");
  assert.equal(item.id, "agent-call:code-builder");
  // 面板行靠这个色画工牌（与侧栏同一枚）：无色档案不给 data.agentColor，由行内哈希兜底。
  assert.deepEqual(item.data, { agentColor: "green" });
  assert.deepEqual(
    mapProjectAgentsToCallMentionItems([directory[0]!], "doc")[0]!.data,
    {},
    "档案没选色就不带 agentColor 键",
  );
});
