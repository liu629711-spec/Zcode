import type { AssetManifest } from "../types.js";

/**
 * 渐变流字（自制原创）：background-clip:text + background-position 线性循环，
 * 渐变首尾同色保证无缝流动。图纸单文件自包含，内容与 preview 同一份。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>渐变流字</title>
<style>
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .flow {
    margin: 0; font-size: clamp(30px, 6vw, 52px); font-weight: 800; letter-spacing: 0.02em;
    background: linear-gradient(100deg, #67e8f9, #a5b4fc, #f0abfc, #67e8f9);
    background-size: 200% 100%;
    -webkit-background-clip: text; background-clip: text; color: transparent;
    animation: flow 6s linear infinite;
  }
  @keyframes flow { from { background-position: 0% 0; } to { background-position: 200% 0; } }
  @media (prefers-reduced-motion: reduce) {
    .flow { animation: none; background-position: 20% 0; }
  }
</style>
</head>
<body>
  <h1 class="flow">让文字自己发光</h1>
</body>
</html>`;

export const gradientFlowTextAsset: AssetManifest = {
  id: "gradient-flow-text",
  title: "渐变流字",
  titleEn: "Gradient Flow Text",
  description: "四色渐变在字形内无缝流动，标题自带高光质感。",
  descriptionEn: "A seamless four-color gradient flowing inside the glyphs.",
  category: "text-animation",
  tags: ["css", "文字动效", "渐变", "标题", "暗色"],
  previewHtml: HTML,
  files: [{ name: "gradient-flow-text.html", language: "html", content: HTML }],
  prompt:
    "请把「渐变流字」装进我的项目：一个渐变流动的标题组件——linear-gradient 四色循环（青→蓝紫→粉回到青），background-clip: text 让渐变只在字形内显示，background-position 线性循环形成无缝流动，首尾同色保证循环不跳变；尊重 prefers-reduced-motion（降级为静止渐变）。先看现有标题的字号字重层级，作为标题变体融入，不要覆盖全局排版。",
};
