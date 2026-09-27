import type { AssetManifest } from "../types.js";

/**
 * 霓虹按钮（自制原创）：透明底 + 青色描边 + 内外双层辉光，hover 增辉并轻闪。
 * 纯 CSS，图纸分 html+css 两件可拿走；preview 是同款内联版。
 */

const CSS = `* { box-sizing: border-box; margin: 0; }
body {
  min-height: 100vh; display: grid; place-items: center;
  background: radial-gradient(110% 110% at 50% 0%, #0d1424 0%, #080d18 60%, #060910 100%);
  font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
}
.neon {
  padding: 12px 32px; border-radius: 10px; cursor: pointer;
  background: transparent; color: #67e8f9;
  font-size: 15px; font-weight: 600; letter-spacing: 0.1em;
  border: 1.5px solid #22d3ee;
  box-shadow: 0 0 8px rgb(34 211 238 / 0.5), inset 0 0 8px rgb(34 211 238 / 0.25);
  transition: box-shadow 0.25s ease, color 0.25s ease;
}
.neon:hover {
  color: #a5f3fc;
  box-shadow: 0 0 18px rgb(34 211 238 / 0.85), inset 0 0 14px rgb(34 211 238 / 0.4);
  text-shadow: 0 0 6px rgb(103 232 249 / 0.8);
  animation: neon-flicker 1.6s step-end infinite;
}
.neon:focus-visible { outline: 2px solid #67e8f9; outline-offset: 3px; }
@keyframes neon-flicker {
  0%, 100% { opacity: 1; }
  12% { opacity: 0.82; } 16% { opacity: 1; }
  58% { opacity: 0.9; } 62% { opacity: 1; }
}
@media (prefers-reduced-motion: reduce) {
  .neon:hover { animation: none; }
}`;

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>霓虹按钮</title>
<style>${CSS}</style>
</head>
<body>
  <button class="neon" type="button">NEON ENTER</button>
</body>
</html>`;

export const neonButtonAsset: AssetManifest = {
  id: "neon-button",
  title: "霓虹按钮",
  titleEn: "Neon Button",
  description: "描边加双层辉光的赛博霓虹按钮，hover 会轻闪。",
  descriptionEn: "An outlined cyber button with double glow and a hover flicker.",
  category: "control",
  tags: ["css", "按钮", "霓虹", "暗色"],
  previewHtml: PREVIEW_HTML,
  files: [
    {
      name: "index.html",
      language: "html",
      content: `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="stylesheet" href="neon-button.css">
</head>
<body>
  <button class="neon" type="button">NEON ENTER</button>
</body>
</html>`,
    },
    { name: "neon-button.css", language: "css", content: CSS },
  ],
  prompt:
    "请把「霓虹按钮」装进我的项目：一个赛博霓虹风格按钮——透明底加青色 1.5px 描边、内外双层辉光（box-shadow），hover 时辉光增强、文字带光晕并轻微闪烁一下；保留 focus-visible 描边；只在深色底上使用。先看现有的按钮变体体系，作为特殊强调按钮融入，不要动主按钮规范。",
};
