import type { AssetManifest } from "../types.js";

/**
 * 「空状态三变体」口令卡（自制原创）：prompt 类货——口令本身就是货。
 */

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>空状态三变体 · 口令卡</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: linear-gradient(150deg, #14121f 0%, #221d33 55%, #100e1a 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #dbe4f0;
  }
  .note {
    width: min(420px, 88vw); padding: 28px 30px; border-radius: 20px;
    background: rgb(255 255 255 / 0.06); border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(14px);
  }
  h1 { margin: 0 0 6px; font-size: 20px; }
  .kind { font-size: 12px; color: #f0abfc; letter-spacing: 0.12em; }
  p { margin: 14px 0 0; font-size: 14px; line-height: 1.8; color: #aebad0; }
  b { color: #e6edf5; }
</style>
</head>
<body>
  <div class="note">
    <div class="kind">口令卡 · PROMPT</div>
    <h1>空状态三变体</h1>
    <p>这是一件<b>口令货</b>：不附带代码图纸，<b>口令本身就是货</b>。</p>
    <p>复制或发进会话后，智能体会按口令现做统一的 EmptyState 组件：无数据、搜索无结果、加载失败三种变体一次给全，插画位留成插槽。</p>
    <p>适合放在<b>新会话</b>里直接点单，文案与动作可再按你的业务微调。</p>
  </div>
</body>
</html>`;

export const emptyStateAsset: AssetManifest = {
  id: "empty-state-prompt",
  title: "空状态三变体",
  titleEn: "Empty States Trio",
  description: "口令卡：无数据/无结果/失败三种空状态一套口令搞定。",
  descriptionEn: "A prompt card covering empty, no-result and error states.",
  category: "prompt",
  tags: ["prompt", "空状态", "组件", "口令"],
  previewHtml: PREVIEW_HTML,
  files: [],
  prompt:
    "请为我的项目做一套「空状态组件」：统一的 EmptyState 组件出三种变体——无数据（插画位 + 「这里还什么都没有」+ 主行动按钮「新建」）、搜索无结果（放大镜位 + 「没有找到相关内容」+ 「清除搜索」文字按钮）、加载失败（断线位 + 「加载失败了」+ 「重试」按钮）；插画位做成插槽方便替换；垂直居中、间距宽松、文案两行以内。要求：暗色底适配、按钮接现有动作体系、插画用纯 CSS/SVG 自绘不引外部图片；先看现有的列表与错误处理约定，融入而不是覆盖。",
};
