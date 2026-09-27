import type { AssetManifest } from "../types.js";

/**
 * 果冻开关（自制原创）：纯 CSS toggle，回弹用 overshoot 贝塞尔 + 落位果冻抖动。
 * 图纸分 html+css 两件可拿走；preview 是同款内联版。
 */

const CSS = `* { box-sizing: border-box; margin: 0; }
body {
  min-height: 100vh; display: grid; place-items: center;
  background: radial-gradient(120% 120% at 50% 0%, #131c2b 0%, #0b101a 60%, #080c14 100%);
  font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
}
.jelly-toggle { position: relative; display: inline-block; }
.jelly-toggle input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
.track {
  display: block; width: 88px; height: 46px; border-radius: 999px;
  background: #1c2534;
  box-shadow: inset 0 2px 8px rgb(0 0 0 / 0.65), inset 0 -1px 0 rgb(255 255 255 / 0.06);
  transition: background 0.35s ease;
}
.thumb {
  position: absolute; top: 5px; left: 5px; width: 36px; height: 36px; border-radius: 50%;
  background: radial-gradient(circle at 32% 28%, #f4f8ff, #aebfd8 70%);
  box-shadow: 0 4px 10px rgb(0 0 0 / 0.5);
  transition: transform 0.4s cubic-bezier(0.2, 1.8, 0.4, 1); /* overshoot = 果冻回弹 */
}
.jelly-toggle input:checked + .track { background: linear-gradient(135deg, #34d399, #0ea5e9); }
.jelly-toggle input:checked + .track .thumb { transform: translateX(42px); animation: jelly 0.45s ease; }
.jelly-toggle input:focus-visible + .track { outline: 2px solid #60a5fa; outline-offset: 3px; }
@keyframes jelly {
  0% { scale: 1 1; } 30% { scale: 1.25 0.8; } 55% { scale: 0.85 1.15; } 75% { scale: 1.08 0.94; } 100% { scale: 1 1; }
}
@media (prefers-reduced-motion: reduce) {
  .track, .thumb { transition: none; }
  .jelly-toggle input:checked + .track .thumb { animation: none; }
}`;

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>果冻开关</title>
<style>${CSS}</style>
</head>
<body>
  <label class="jelly-toggle">
    <input type="checkbox" checked aria-label="果冻开关">
    <span class="track"><span class="thumb"></span></span>
  </label>
</body>
</html>`;

export const jellyToggleAsset: AssetManifest = {
  id: "jelly-toggle",
  title: "果冻开关",
  description: "纯 CSS 开关：拨动时带 overshoot 回弹和果冻抖动，暗色精致。",
  category: "control",
  tags: ["css", "开关", "动效", "表单"],
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
<link rel="stylesheet" href="jelly-toggle.css">
</head>
<body>
  <label class="jelly-toggle">
    <input type="checkbox" checked aria-label="果冻开关">
    <span class="track"><span class="thumb"></span></span>
  </label>
</body>
</html>`,
    },
    { name: "jelly-toggle.css", language: "css", content: CSS },
  ],
  prompt:
    "请把「果冻开关」装进我的项目：一个纯 CSS 的 toggle 开关，拨动时滑块带 overshoot 回弹（cubic-bezier 过冲）和一次 squash-stretch 果冻抖动；选中态轨道变青绿渐变，暗色底精致；要保留可访问性（真实 checkbox、focus-visible 描边）并尊重 prefers-reduced-motion。先看现有表单组件风格，融入而不是覆盖。",
};
