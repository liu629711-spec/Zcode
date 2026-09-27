import type { AssetManifest } from "../types.js";

/**
 * 流动波浪（自制原创）：canvas 三层不同振幅/相位的正弦波透叠起伏。
 * 图纸分 html+js 两件可拿走；preview 是同款内联版。
 */

const JS = `var canvas = document.getElementById("sea");
var ctx = canvas.getContext("2d");
var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
var w = 0, h = 0, t = 0;
var layers = [
  { amp: 24, len: 0.010, speed: 0.9, base: 0.58, color: "rgba(56, 189, 248, 0.30)" },
  { amp: 18, len: 0.014, speed: 1.4, base: 0.68, color: "rgba(129, 140, 248, 0.28)" },
  { amp: 12, len: 0.019, speed: 2.0, base: 0.78, color: "rgba(45, 212, 191, 0.24)" }
];
function resize() { w = canvas.width = innerWidth; h = canvas.height = innerHeight; }
addEventListener("resize", resize);
resize();
function paint() {
  var sky = ctx.createLinearGradient(0, 0, 0, h);
  sky.addColorStop(0, "#060b16"); sky.addColorStop(1, "#0c1526");
  ctx.fillStyle = sky; ctx.fillRect(0, 0, w, h);
  for (var i = 0; i < layers.length; i++) {
    var L = layers[i];
    ctx.beginPath(); ctx.moveTo(0, h);
    for (var x = 0; x <= w; x += 4) {
      var y = h * L.base
        + Math.sin(x * L.len + t * L.speed) * L.amp
        + Math.sin(x * L.len * 0.53 + t * L.speed * 1.6) * L.amp * 0.5;
      ctx.lineTo(x, y);
    }
    ctx.lineTo(w, h); ctx.closePath();
    ctx.fillStyle = L.color; ctx.fill();
  }
}
if (reduce) paint();
else requestAnimationFrame(function loop() { t += 0.016; paint(); requestAnimationFrame(loop); });`;

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>流动波浪</title>
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; }
  body { background: #060b16; }
  canvas { display: block; width: 100%; height: 100%; }
</style>
</head>
<body>
  <canvas id="sea"></canvas>
  <script>${JS}</script>
</body>
</html>`;

export const waveBackgroundAsset: AssetManifest = {
  id: "wave-background",
  title: "流动波浪",
  titleEn: "Flowing Waves",
  description: "三层正弦波透叠起伏的深海 canvas 背景。",
  descriptionEn: "Three translucent sine waves rolling over a deep-sea canvas.",
  category: "background",
  tags: ["canvas", "背景", "波浪", "氛围"],
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
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; }
  body { background: #060b16; }
  canvas { display: block; width: 100%; height: 100%; }
</style>
</head>
<body>
  <canvas id="sea"></canvas>
  <script src="wave-background.js"></script>
</body>
</html>`,
    },
    { name: "wave-background.js", language: "javascript", content: JS },
  ],
  prompt:
    "请把「流动波浪背景」装进我的项目：一块全屏 canvas 背景——深海渐变底色上三层不同振幅与相位的正弦波从左到右流动、互相透叠（青蓝/蓝紫/青绿半透明），营造缓慢起伏的海面；requestAnimationFrame 驱动、resize 自适应；尊重 prefers-reduced-motion（降级为静止海面）。内容层叠在上面保证可读。先看现有页面结构，融入而不是覆盖。",
};
