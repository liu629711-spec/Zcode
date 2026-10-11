// ============================================================
// 员工技能册（2026-10-04 学习沉淀 v1，Hermes「经验自建技能」同款）：
// 技能 = 随身柜 skills/ 目录里的一张卡（一个 .md：frontmatter name/description +
// 正文写"什么时候用/步骤/坑/老板对成品的偏好"）。两处消费：
// - 注入：会话 context 初始化时把技能索引（名 + 一句话简介 + 文件路径）拼进
//   persona system prompt（Hermes 两段式渐进披露的 v1——正文员工用 Read 自己打开，
//   不做 skill_view 专用工具）；索引是会话开局冻结的，本轮新写的卡下个会话生效
//   （Hermes 同款取舍，保 prefix cache）。
// - 沉淀：每单交活后的复盘轮（work-orders.ts）把"这类活怎么干"写成新卡或补旧卡。
// 跟人走（D1 口径）：技能在随身柜，项目事实归工作区本子、老板是谁归随身本子
// （双层记忆既有规矩，一事只进一库）。
// ============================================================

import { basename, join } from "node:path";
import type { FileSystemPort } from "@zcode/contracts";
import { neutralizeEnvelopeTags } from "./work-order.js";

/** 技能卡目录名（随身柜下的固定子目录）。 */
export const AGENT_SKILLS_DIR_NAME = "skills";

/** 随身柜 → 技能册根。 */
export function resolveAgentSkillsRoot(personalNotebookRoot: string): string {
  return join(personalNotebookRoot, AGENT_SKILLS_DIR_NAME);
}

/** 索引里一句话简介的上限（Hermes SKILL_PROMPT_DESC_LIMIT 同值：索引靠它可扫）。 */
export const SKILL_DESCRIPTION_MAX_CHARS = 60;

/** 技能索引总体字符预算（对齐引擎技能元数据的 20k 口径）。 */
export const AGENT_SKILLS_INDEX_BUDGET_CHARS = 20_000;

export interface AgentSkillCard {
  /** 文件名去 .md（技能卡 slug），注入时给 Read 用的就是它拼出的路径。 */
  slug: string;
  name: string;
  description: string;
}

/**
 * 解析一张技能卡：frontmatter 只认单行 name/description（不引 yaml 库，卡是员工
 * 自己写的，规矩在复盘提示词里教）；缺 name 用文件名兜底；description 超长截断
 * （超限的卡也进索引——正文才是主菜，简介剪短即可）；正文为空的卡不进索引。
 */
export function parseSkillCard(input: { fileName: string; content: string }): AgentSkillCard | undefined {
  const slug = basename(input.fileName).replace(/\.md$/i, "");
  if (!slug) return undefined;
  const frontmatter = input.content.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  let name = slug;
  let description = "";
  if (frontmatter) {
    for (const line of frontmatter[1]!.split(/\r?\n/)) {
      const nameMatch = line.match(/^name:\s*(.+)$/);
      if (nameMatch) name = nameMatch[1]!.trim().replace(/^["']|["']$/g, "") || slug;
      const descriptionMatch = line.match(/^description:\s*(.+)$/);
      if (descriptionMatch) {
        description = descriptionMatch[1]!.trim().replace(/^["']|["']$/g, "");
      }
    }
  }
  if (description.length > SKILL_DESCRIPTION_MAX_CHARS) {
    description = `${description.slice(0, SKILL_DESCRIPTION_MAX_CHARS - 1)}…`;
  }
  if (input.content.trim().length === 0) return undefined;
  return { slug, name, description };
}

/**
 * 技能索引块（拼进 persona system prompt）：一张卡一行 = 名字 + 一句话简介，
 * 文件路径当场给出（员工用 Read 打开）。空册返回 undefined（不注空段）；
 * 超预算截断并如实标注（其余卡仍在目录里，只是索引没列）。
 */
export function buildAgentSkillsIndexPrompt(input: {
  skillsRoot: string;
  cards: readonly AgentSkillCard[];
}): string | undefined {
  if (input.cards.length === 0) return undefined;
  const lines: string[] = [
    "## 技能册（这类活怎么干的手艺卡，跟人走）",
    "遇到与下列技能相关的任务，先用 Read 打开对应文件照着做再动手，拿不准是否相关时宁可打开看一眼；没有对口的就按任务本身干：",
  ];
  let budget = AGENT_SKILLS_INDEX_BUDGET_CHARS;
  let truncated = false;
  for (const card of input.cards) {
    const line = `- ${card.name}（${join(input.skillsRoot, `${card.slug}.md`)}）：${card.description || "（无简介）"}`;
    if (line.length > budget) {
      truncated = true;
      break;
    }
    lines.push(line);
    budget -= line.length;
  }
  if (truncated) {
    lines.push("（技能册过大，索引已截断——其余技能卡仍在 skills/ 目录里，按文件名用 Read 探索。）");
  }
  return lines.join("\n");
}

/**
 * 读技能册 → 索引块。读路径不现造目录（D25 口径）：册子不存在就是空，不注不建。
 * 单卡读取失败跳过该卡（一张坏卡不拖垮整册）。
 */
export async function loadAgentSkillsIndexPrompt(input: {
  fileSystemPort: FileSystemPort | undefined;
  personalNotebookRoot: string;
}): Promise<string | undefined> {
  if (!input.fileSystemPort) return undefined;
  const skillsRoot = resolveAgentSkillsRoot(input.personalNotebookRoot);
  let fileNames: string[];
  try {
    const listing = await input.fileSystemPort.listDirectory({ path: skillsRoot });
    fileNames = (listing.entries ?? [])
      .map((entry) => entry.name)
      .filter((name) => typeof name === "string" && name.toLowerCase().endsWith(".md"));
  } catch {
    return undefined;
  }
  const cards: AgentSkillCard[] = [];
  for (const fileName of fileNames.sort()) {
    try {
      const content = (
        await input.fileSystemPort.readTextFile({ path: join(skillsRoot, fileName) })
      ).content;
      const card = parseSkillCard({ fileName, content });
      if (card) cards.push(card);
    } catch {
      // 单卡坏掉跳过。
    }
  }
  return buildAgentSkillsIndexPrompt({ skillsRoot, cards });
}

/**
 * 复盘轮提示词（Hermes _SKILL_REVIEW_PROMPT 的忠实中文浓缩 + 三库路由）。
 * 失败口径（学习沉淀跟进 2026-10-05）：失败单也复盘——沉淀的是「下次怎么提前
 * 识别并避开这个坑」，不是把失败原因抄一遍；其余纪律与成功复盘完全同款。
 */
export function buildWorkOrderDebriefText(input: {
  skillsRoot: string;
  failure?: boolean;
  failureReason?: string;
}): string {
  const failureReason =
    input.failureReason
      ?.replace(/\s+/g, " ")
      .trim()
      .slice(0, 200) ?? "";
  if (input.failure === true) {
    return [
      "复盘（自动收工学习，不是用户发言）：",
      `这一单你没有干成${failureReason ? `（原因：${failureReason}）` : ""}。花一段回顾这次失败，把值得长期保留的教训写进你的技能册：${input.skillsRoot}`,
      "一个技能一个 .md 文件；frontmatter 写两行 name 与 description（description 一句话、不超过 60 字，索引靠它可搜）；正文写：什么时候用 / 步骤 / 坑 / 老板对成品的偏好。",
      "失败复盘的重点：下次怎么提前识别这个坑、接到这类活第一步先查什么、哪条路走不通该在哪一步换路——不是把失败原因抄一遍。",
      "分流纪律（一件事只进一本账，绝不两头写）：",
      "- 这类活怎么干、步骤、坑、老板对产出口味 → 技能册。已有一张对口的卡就先用 Read 打开它，在原卡上补或改，不建重复卡；",
      "- 这个项目/环境的事实（路径、约定、坑）→ 写进工作区记事本（既有规矩）；",
      "- 老板是谁、跨项目的偏好 → 写进随身记事本 MEMORY.md（既有规矩）。",
      "禁止沉淀：环境性失败（没装的包、没配的凭据）、对工具的负面断言（「X 工具是坏的」）、一次性任务的过程流水账——这些不是本事。",
      "实在没有值得沉淀的，就只回复「无事可记」四个字，不要硬凑。",
    ].join("\n");
  }
  return [
    "复盘（自动收工学习，不是用户发言）：",
    `这一单你刚交完活。花一段回顾这次干活的过程，把值得长期保留的经验写进你的技能册：${input.skillsRoot}`,
    "一个技能一个 .md 文件；frontmatter 写两行 name 与 description（description 一句话、不超过 60 字，索引靠它可搜）；正文写：什么时候用 / 步骤 / 坑 / 老板对成品的偏好。",
    "分流纪律（一件事只进一本账，绝不两头写）：",
    "- 这类活怎么干、步骤、坑、老板对产出口味 → 技能册。已有一张对口的卡就先用 Read 打开它，在原卡上补或改，不建重复卡；",
    "- 这个项目/环境的事实（路径、约定、坑）→ 写进工作区记事本（既有规矩）；",
    "- 老板是谁、跨项目的偏好 → 写进随身记事本 MEMORY.md（既有规矩）。",
    "禁止沉淀：环境性失败（没装的包、没配的凭据）、对工具的负面断言（「X 工具是坏的」）、一次性任务的过程流水账——这些不是本事。",
    "实在没有值得沉淀的，就只回复「无事可记」四个字，不要硬凑。",
  ].join("\n");
}

// ── 派单自动触发（学习沉淀 v1.5）：接活时对任务正文现读现匹配 ──────────
// 索引是会话开局冻结的（保 prefix cache），代价有二：①模型可能没把任务和索引
// 对上号；②本轮复盘刚写的卡要下个会话才进索引。派单时对任务正文现读技能册、
// 命中卡整卡带上——两个代价一次收掉。参照件核对：Hermes skill_utils.py:761 与
// 我们 v1 同款索引披露、无按单匹配；Claude Code 的 Skill 工具与我们的 Read
// 路径同效——这刀是补自己的天花板，不是抄谁。

/** 单卡正文注入上限：超限截断并标注，完整卡仍在文件里可 Read。 */
export const TASK_MATCHED_SKILL_CHAR_CAP = 2_000;
/** 单张工单最多自动带几张卡（命中再多也只带前三，防信封无界膨胀）。 */
export const TASK_MATCHED_SKILL_MAX_CARDS = 3;

/**
 * 任务正文命中哪些卡。无分词库不做语义，召回靠三层字面信号（宁带勿漏——
 * 漏了功能看着就是死的；误命中由「至多带三张」与卡内容本身兜底）：
 * ①卡名整体出现在任务里（最强信号）；②slug 大小写不敏感出现；③卡名里的
 * 中文二字窗（「周报写作」→ 周报/报写/写作）任一出现在任务里——老板说
 * 「写个周报」也要能对上「周报写作」这张卡。description 自由文本不参与
 * （没有分段器，误召回更吵）；单字名不参与（到处误命中）。
 */
export function selectTaskMatchedSkills(input: {
  taskText: string;
  cards: readonly AgentSkillCard[];
  maxCards?: number;
}): AgentSkillCard[] {
  const task = input.taskText;
  if (task.trim().length === 0 || input.cards.length === 0) return [];
  const max = input.maxCards ?? TASK_MATCHED_SKILL_MAX_CARDS;
  const lowered = task.toLowerCase();
  const nameHitsTask = (name: string): boolean => {
    const cleaned = name.trim();
    if (cleaned.length < 2) return false;
    if (lowered.includes(cleaned.toLowerCase())) return true;
    // 中文二字窗只对汉字段开窗（混排名里的 ascii 段会产生垃圾窗）。
    for (const run of cleaned.match(/[\u4e00-\u9fff]{2,}/g) ?? []) {
      for (let i = 0; i + 2 <= run.length; i++) {
        if (lowered.includes(run.slice(i, i + 2))) return true;
      }
    }
    return false;
  };
  const hits: AgentSkillCard[] = [];
  for (const card of input.cards) {
    const byName = nameHitsTask(card.name);
    const bySlug =
      card.slug.length >= 2 &&
      lowered.includes(card.slug.toLowerCase()) &&
      card.slug !== card.name.trim();
    if (byName || bySlug) {
      hits.push(card);
      if (hits.length >= max) break;
    }
  }
  return hits;
}

/**
 * 命中卡正文块（拼进工单信封 </work-order> 之后、压轴执行要求之前——与评审
 * 要求同一槽位：标签内原文会被事件面当任务正文透出，里面不能掺指令的纪律
 * 照旧；卡正文里的同形信封标签一律中和，防伪造信封边界）。
 */
export function buildTaskMatchedSkillsPrompt(input: {
  skillsRoot: string;
  matches: ReadonlyArray<AgentSkillCard & { content: string }>;
}): string | undefined {
  if (input.matches.length === 0) return undefined;
  const lines: string[] = [
    "本单命中你的技能册，下面整卡附上（照卡办事；截断的卡用 Read 打开完整文件）：",
  ];
  for (const card of input.matches) {
    const path = join(input.skillsRoot, `${card.slug}.md`);
    const content = neutralizeEnvelopeTags(card.content).trim();
    const body =
      content.length > TASK_MATCHED_SKILL_CHAR_CAP
        ? `${content.slice(0, TASK_MATCHED_SKILL_CHAR_CAP)}…（卡过长已截断，完整内容用 Read 打开：${path}）`
        : content;
    lines.push(`### ${card.name}（${path}）`, body);
  }
  return lines.join("\n");
}

/**
 * 现读技能册 → 命中卡正文块。册子不存在/读不了=无命中（读路径不现造目录，
 * D25 口径）；单卡坏掉跳过。每单都现读（不缓存）——新写的卡下一单就生效。
 */
export async function loadTaskMatchedSkillsPrompt(input: {
  fileSystemPort: FileSystemPort | undefined;
  personalNotebookRoot: string;
  taskText: string;
}): Promise<string | undefined> {
  if (!input.fileSystemPort) return undefined;
  const skillsRoot = resolveAgentSkillsRoot(input.personalNotebookRoot);
  let fileNames: string[];
  try {
    const listing = await input.fileSystemPort.listDirectory({ path: skillsRoot });
    fileNames = (listing.entries ?? [])
      .map((entry) => entry.name)
      .filter((name) => typeof name === "string" && name.toLowerCase().endsWith(".md"));
  } catch {
    return undefined;
  }
  const matches: Array<AgentSkillCard & { content: string }> = [];
  for (const fileName of fileNames.sort()) {
    if (matches.length >= TASK_MATCHED_SKILL_MAX_CARDS) break;
    try {
      const content = (
        await input.fileSystemPort.readTextFile({ path: join(skillsRoot, fileName) })
      ).content;
      const card = parseSkillCard({ fileName, content });
      if (!card) continue;
      if (selectTaskMatchedSkills({ taskText: input.taskText, cards: [card] }).length > 0) {
        matches.push({ ...card, content });
      }
    } catch {
      // 单卡坏掉跳过。
    }
  }
  return buildTaskMatchedSkillsPrompt({ skillsRoot, matches });
}
