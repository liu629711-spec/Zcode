// ============================================================
// 员工技能册（学习沉淀 v1）纯规则的可运行检查。
// 运行：node --test apps/zcode-cli/packages/core/src/subagent/agent-skills.test.ts
// （Node 24 类型剥离直接跑；源内 .js 说明符指向 .ts 文件时用 tsc+node 路数，见记忆）
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  buildAgentSkillsIndexPrompt,
  buildWorkOrderDebriefText,
  loadAgentSkillsIndexPrompt,
  parseSkillCard,
  resolveAgentSkillsRoot,
  SKILL_DESCRIPTION_MAX_CHARS,
} from "./agent-skills.js";

const SKILLS_ROOT = "C:\\Users\\me\\.zcode\\agent-memory\\a-uuid\\skills";

test("目录推导：随身柜 + skills 固定子目录", () => {
  assert.equal(resolveAgentSkillsRoot("C:\\notebook"), "C:\\notebook\\skills");
});

test("技能卡解析：frontmatter 取名与简介，缺名用文件名兜底，超长简介截断", () => {
  const card = parseSkillCard({
    fileName: "frontend-design.md",
    content: "---\nname: 前端设计\ndescription: 写界面前先摸主题，字体不超过两套，动效克制\n---\n\n正文步骤……",
  });
  assert.equal(card?.name, "前端设计");
  assert.equal(card?.slug, "frontend-design");
  assert.equal(card?.description, "写界面前先摸主题，字体不超过两套，动效克制");

  const fallback = parseSkillCard({ fileName: "sql-tuning.md", content: "---\n---\n正文" });
  assert.equal(fallback?.name, "sql-tuning");

  const long = parseSkillCard({
    fileName: "long.md",
    content: `---\ndescription: ${"长".repeat(80)}\n---\n正文`,
  });
  assert.equal(long?.description.length, SKILL_DESCRIPTION_MAX_CHARS);
  assert.ok(long?.description.endsWith("…"));

  assert.equal(parseSkillCard({ fileName: "empty.md", content: "   " }), undefined);
});

test("索引块：空册不注入；有卡给路径；超预算截断并如实标注", () => {
  assert.equal(buildAgentSkillsIndexPrompt({ skillsRoot: SKILLS_ROOT, cards: [] }), undefined);

  const prompt = buildAgentSkillsIndexPrompt({
    skillsRoot: SKILLS_ROOT,
    cards: [{ slug: "frontend-design", name: "前端设计", description: "先摸主题" }],
  });
  assert.ok(prompt?.includes("## 技能册"));
  assert.ok(prompt?.includes(`${SKILLS_ROOT}\\frontend-design.md`), "路径当场给出，员工 Read 就能打开");
  assert.ok(prompt?.includes("先摸主题"));

  const big = buildAgentSkillsIndexPrompt({
    skillsRoot: SKILLS_ROOT,
    cards: Array.from({ length: 400 }, (_, index) => ({
      slug: `skill-${index}`,
      name: `技能${index}`,
      description: "一句话简介".repeat(30),
    })),
  });
  assert.ok(big?.includes("索引已截断"), "超预算如实标注，不静默丢卡");
});

test("复盘提示词：给技能册路径、三库分流、禁沉淀清单与「无事可记」逃生口", () => {
  const text = buildWorkOrderDebriefText({ skillsRoot: SKILLS_ROOT });
  assert.ok(text.includes(SKILLS_ROOT));
  assert.ok(text.includes("一件事只进一本账"));
  assert.ok(text.includes("禁止沉淀"));
  assert.ok(text.includes("无事可记"));
  assert.ok(text.includes("description"), "frontmatter 规矩要教（索引靠简介可搜）");
});

test("读册→索引：列目录+逐卡读取；坏卡跳过、目录缺席返回 undefined", async () => {
  const calls: string[] = [];
  const port = {
    listDirectory: async ({ path }: { path: string }) => {
      calls.push(`list:${path}`);
      return {
        entries: [{ name: "good.md" }, { name: "broken.md" }, { name: "not-md.txt" }],
        path,
      };
    },
    readTextFile: async ({ path }: { path: string }) => {
      calls.push(`read:${path}`);
      if (path.endsWith("broken.md")) throw new Error("bad card");
      return { content: `---\nname: 好卡\ndescription: d\n---\n正文` };
    },
  };
  const prompt = await loadAgentSkillsIndexPrompt({
    fileSystemPort: port as never,
    personalNotebookRoot: "C:\\notebook",
  });
  assert.ok(prompt?.includes("好卡"));
  assert.ok(calls.some((call) => call === "list:C:\\notebook\\skills"));
  assert.equal(calls.filter((call) => call.startsWith("read:")).length, 2, "txt 不读、坏卡读了但跳过");

  const missing = await loadAgentSkillsIndexPrompt({
    fileSystemPort: {
      listDirectory: async () => {
        throw new Error("no dir");
      },
      readTextFile: async () => {
        throw new Error("never");
      },
    } as never,
    personalNotebookRoot: "C:\\notebook",
  });
  assert.equal(missing, undefined, "读路径不现造目录，册子不存在就是空");
});

test("简介上限常量对齐 Hermes（60 字）", () => {
  assert.equal(SKILL_DESCRIPTION_MAX_CHARS, 60);
});
