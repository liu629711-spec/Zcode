import type { AssetManifest } from "../types.js";

/**
 * 涟漪按钮（自制原创）：点击处生成扩散波纹，键盘触发从中心起波。
 * 图纸分 html+js 两件可拿走；preview 是同款内联版。
 */

const JS = `var btn = document.querySelector(".ripple");
var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
btn.addEventListener("click", function (e) {
  if (reduce) return; // 降级：只保留按压反馈，不产生波纹
  var rect = btn.getBoundingClientRect();
  var d = Math.max(rect.width, rect.height) * 2;
  // 键盘触发（Enter/空格）没有指针坐标，从按钮中心起波
  var x = e.clientX || rect.left + rect.width / 2;
  var y = e.clientY || rect.top + rect.height / 2;
  var wave = document.createElement("span");
  wave.className = "wave";
  wave.style.width = wave.style.height = d + "px";
  wave.style.left = x - rect.left - d / 2 + "px";
  wave.style.top = y - rect.top - d / 2 + "px";
  btn.appendChild(wave);
  wave.addEventListener("animationend", function () { wave.remove(); });
});`;

const CSS = `* { box-sizing: border-box; margin: 0; }
body {
  min-height: 100%; display: grid; place-items: center;
  background: radial-gradient(110% 110% at 50% 10%, #101827 0%, #0a0f18 65%, #070b12 100%);
  font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
}
.ripple {
  position: relative; overflow: hidden; padding: 13px 36px; border: 0; border-radius: 12px;
  cursor: pointer; font-size: 15px; font-weight: 600; letter-spacing: 0.06em; color: #08131f;
  background: linear-gradient(135deg, #38bdf8, #6366f1);
  box-shadow: 0 6px 18px rgb(56 130 246 / 0.35);
}
.ripple:active { transform: scale(0.98); }
.ripple:focus-visible { outline: 2px solid #93c5fd; outline-offset: 3px; }
.wave {
  position: absolute; border-radius: 50%; pointer-events: none;
  background: rgb(255 255 255 / 0.35);
  transform: scale(0); animation: wave 0.6s ease-out forwards;
}
@keyframes wave { to { transform: scale(1); opacity: 0; } }`;

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>涟漪按钮</title>
<style>${CSS}</style>
</head>
<body>
  <button class="ripple" type="button">点我一下</button>
  <script>${JS}</script>
</body>
</html>`;

export const rippleButtonAsset: AssetManifest = {
  id: "ripple-button",
  title: "涟漪按钮",
  titleEn: "Ripple Button",
  description: "点击处扩散水波纹的按钮，键盘触发从中心起波。",
  descriptionEn: "A button that ripples from the click point, center for keyboard.",
  category: "control",
  tags: ["js", "按钮", "涟漪", "反馈"],
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
<link rel="stylesheet" href="ripple-button.css">
</head>
<body>
  <button class="ripple" type="button">点我一下</button>
  <script src="ripple-button.js"></script>
</body>
</html>`,
    },
    { name: "ripple-button.css", language: "css", content: CSS },
    { name: "ripple-button.js", language: "javascript", content: JS },
  ],
  prompt:
    "请把「涟漪按钮」装进我的项目：一个点击扩散涟漪的主按钮——按下时从点击坐标生成圆形波纹（overflow 裁剪），scale 从 0 放大同时淡出，动画结束移除节点；键盘触发（Enter/空格）时从按钮中心扩散；同一按钮可叠加多个波纹；尊重 prefers-reduced-motion（不产生波纹，只保留按压反馈）。先看现有按钮体系，融入而不是覆盖。",
};
