import type { AssetManifest } from "../types.js";

/**
 * 「落地页首幅」口令卡（自制原创）：prompt 类货——没有代码图纸，口令本身就是货。
 * preview 是一张简洁说明页。
 */

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>落地页首幅 · 口令卡</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: linear-gradient(150deg, #0b1122 0%, #141a36 55%, #090d1c 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #dbe4f0;
  }
  .note {
    width: min(420px, 88vw); padding: 28px 30px; border-radius: 20px;
    background: rgb(255 255 255 / 0.06); border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(14px);
  }
  h1 { margin: 0 0 6px; font-size: 20px; }
  .kind { font-size: 12px; color: #7dd3fc; letter-spacing: 0.12em; }
  p { margin: 14px 0 0; font-size: 14px; line-height: 1.8; color: #aebad0; }
  b { color: #e6edf5; }
</style>
</head>
<body>
  <div class="note">
    <div class="kind">口令卡 · PROMPT</div>
    <h1>落地页首幅</h1>
    <p>这是一件<b>口令货</b>：不附带代码图纸，<b>口令本身就是货</b>。</p>
    <p>复制或发进会话后，智能体会按口令现做一屏深色落地页首幅：渐变高亮大标题、双按钮、网格纹理与三个亮点小卡，入场动效已约定克制。</p>
    <p>适合放在<b>新会话</b>里直接点单，也可改几个词换成你自己的产品信息。</p>
  </div>
</body>
</html>`;

export const landingHeroAsset: AssetManifest = {
  id: "landing-hero-prompt",
  title: "落地页首幅",
  titleEn: "Landing Hero",
  description: "口令卡：一句成套口令，让智能体现做深色落地页首屏。",
  descriptionEn: "A prompt card for a full dark landing hero screen.",
  category: "prompt",
  tags: ["prompt", "落地页", "首屏", "口令"],
  previewHtml: PREVIEW_HTML,
  files: [],
  prompt:
    "请为我的项目做一屏「产品落地页首幅」：深色渐变底（近黑的蓝紫调）加极淡的网格线纹理；居中排版——小字产品名 eyebrow、大标题（其中一个关键词用青→紫渐变文字强调）、一句话副标题、主按钮（实心渐变）与次按钮（描边幽灵）并排；标题和副标题入场做一次 20px 上浮淡入（错开 100ms）；往下再放三个并排的产品亮点小卡（图标位 + 标题 + 一句话）。要求：移动端单列堆叠、动效克制并尊重 prefers-reduced-motion；先看现有的路由、设计令牌与组件库再动手，融入而不是覆盖全局样式。",
};
