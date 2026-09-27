import type { AssetManifest } from "../types.js";
import { renderPromptCard } from "../prompt-card.js";

/**
 * 「落地页首幅」口令卡（自制原创）：prompt 类货——没有代码图纸，口令本身就是货。
 * 说明页用共享模板（V2-3 重做：徽标 + 标题 + 说明 + 口令全文排版页）。
 */

const PROMPT =
  "请为我的项目做一屏「产品落地页首幅」：深色渐变底（近黑的蓝紫调）加极淡的网格线纹理；居中排版——小字产品名 eyebrow、大标题（其中一个关键词用青→紫渐变文字强调）、一句话副标题、主按钮（实心渐变）与次按钮（描边幽灵）并排；标题和副标题入场做一次 20px 上浮淡入（错开 100ms）；往下再放三个并排的产品亮点小卡（图标位 + 标题 + 一句话）。要求：移动端单列堆叠、动效克制并尊重 prefers-reduced-motion；先看现有的路由、设计令牌与组件库再动手，融入而不是覆盖全局样式。";

export const landingHeroAsset: AssetManifest = {
  id: "landing-hero-prompt",
  title: "落地页首幅",
  titleEn: "Landing Hero",
  description: "口令卡：一句成套口令，让智能体现做深色落地页首屏。",
  descriptionEn: "A prompt card for a full dark landing hero screen.",
  category: "prompt",
  tags: ["prompt", "落地页", "首屏", "口令"],
  previewHtml: renderPromptCard(
    { title: "落地页首幅", prompt: PROMPT },
    "不附带代码图纸，<b>口令本身就是货</b>。发进会话后，智能体会现做一屏深色落地页首幅：渐变高亮大标题、双按钮、网格纹理与三个亮点小卡，入场动效已约定克制。",
  ),
  files: [],
  prompt: PROMPT,
};
