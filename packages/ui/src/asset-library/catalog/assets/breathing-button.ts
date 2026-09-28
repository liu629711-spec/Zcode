import type { AssetManifest } from "../types.js";

/**
 * 呼吸灯按钮（自制原创）：主按钮带缓慢呼吸的光环，吸引点击但不吵。
 * 图纸分 html+css 两件可拿走；preview 是同款内联版。
 */

const CSS = `* { box-sizing: border-box; margin: 0; }
body {
  min-height: 100vh; display: grid; place-items: center;
  background: radial-gradient(110% 110% at 50% 10%, #101827 0%, #0a0f18 65%, #070b12 100%);
  font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
}
.breathe {
  padding: 13px 34px; border: 0; border-radius: 12px; cursor: pointer;
  font-size: 15px; font-weight: 600; letter-spacing: 0.06em; color: #04121c;
  background: linear-gradient(135deg, #5eead4, #38bdf8);
  box-shadow: 0 0 0 0 rgb(56 189 248 / 0.45);
  animation: breathe 2.6s ease-in-out infinite;
  transition: filter 0.2s ease;
}
.breathe:hover { filter: brightness(1.08); }
.breathe:active { filter: brightness(0.94); }
.breathe:focus-visible { outline: 2px solid #93c5fd; outline-offset: 3px; }
@keyframes breathe {
  0%, 100% { box-shadow: 0 0 0 0 rgb(56 189 248 / 0.45); }
  50% { box-shadow: 0 0 0 14px rgb(56 189 248 / 0); }
}
@media (prefers-reduced-motion: reduce) {
  .breathe { animation: none; box-shadow: 0 0 0 6px rgb(56 189 248 / 0.12); }
}`;

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>呼吸灯按钮</title>
<style>${CSS}</style>
</head>
<body>
  <button class="breathe" type="button">立即开始</button>
</body>
</html>`;

export const breathingButtonAsset: AssetManifest = {
  id: "breathing-button",
  title: "呼吸灯按钮",
  description: "主行动按钮带一圈缓慢扩散的呼吸光环，克制不闪瞎。",
  category: "control",
  tags: ["css", "按钮", "动效", "光晕"],
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
<link rel="stylesheet" href="breathing-button.css">
</head>
<body>
  <button class="breathe" type="button">立即开始</button>
</body>
</html>`,
    },
    { name: "breathing-button.css", language: "css", content: CSS },
  ],
  prompt:
    "请把「呼吸灯按钮」装进我的项目：一个主行动按钮，青蓝渐变底、圆角 12px，外围有一圈每 2.6 秒缓慢扩散消散的呼吸光环（box-shadow 动画），hover 提亮、active 微暗；保留 focus-visible 描边，尊重 prefers-reduced-motion（光环静止为淡晕）。先看现有的按钮变体体系，作为主按钮变体融入，不要另起一套样式。",
};
