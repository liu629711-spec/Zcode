import type { AssetManifest } from "../types.js";

/**
 * 粒子连线（自制原创）：canvas 微粒漂移，近距粒子间连出透明度随距离衰减的细线。
 * 图纸分 html+js 两件可拿走；preview 是同款内联版。
 */

const JS = `// ponytail: 粒子上限 90，O(n²) 配对连线在这个量级（~4000 对/帧）足够流畅；
// 要上千粒子再换空间网格分桶。
var canvas = document.getElementById("net");
var ctx = canvas.getContext("2d");
var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
var w = 0, h = 0, pts = [];
var LINK = 120;
function resize() {
  w = canvas.width = innerWidth; h = canvas.height = innerHeight;
  var n = Math.min(90, Math.floor((w * h) / 16000));
  pts = [];
  for (var i = 0; i < n; i++) {
    pts.push({
      x: Math.random() * w, y: Math.random() * h,
      vx: (Math.random() - 0.5) * 0.5, vy: (Math.random() - 0.5) * 0.5,
    });
  }
}
addEventListener("resize", resize);
resize();
function paint() {
  ctx.clearRect(0, 0, w, h);
  for (var i = 0; i < pts.length; i++) {
    var p = pts[i];
    p.x += p.vx; p.y += p.vy;
    if (p.x < 0 || p.x > w) p.vx *= -1;
    if (p.y < 0 || p.y > h) p.vy *= -1;
    for (var j = i + 1; j < pts.length; j++) {
      var q = pts[j];
      var dx = p.x - q.x, dy = p.y - q.y, d2 = dx * dx + dy * dy;
      if (d2 < LINK * LINK) {
        ctx.globalAlpha = 1 - Math.sqrt(d2) / LINK;
        ctx.strokeStyle = "#3b82f6";
        ctx.beginPath(); ctx.moveTo(p.x, p.y); ctx.lineTo(q.x, q.y); ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;
    ctx.fillStyle = "#93c5fd";
    ctx.beginPath(); ctx.arc(p.x, p.y, 1.6, 0, Math.PI * 2); ctx.fill();
  }
}
if (reduce) paint();
else requestAnimationFrame(function loop() { paint(); requestAnimationFrame(loop); });`;

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>粒子连线</title>
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; }
  body { background: radial-gradient(120% 120% at 50% 0%, #0b1226 0%, #070b18 60%, #05070f 100%); }
  canvas { display: block; width: 100%; height: 100%; }
</style>
</head>
<body>
  <canvas id="net"></canvas>
  <script>${JS}</script>
</body>
</html>`;

export const particleNetworkAsset: AssetManifest = {
  id: "particle-network",
  title: "粒子连线",
  titleEn: "Particle Network",
  description: "微粒漂移互连成网的 canvas 背景，科技感衬底。",
  descriptionEn: "Drifting particles linked into a breathing network on canvas.",
  category: "background",
  tags: ["canvas", "背景", "粒子", "科技感"],
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
  body { background: radial-gradient(120% 120% at 50% 0%, #0b1226 0%, #070b18 60%, #05070f 100%); }
  canvas { display: block; width: 100%; height: 100%; }
</style>
</head>
<body>
  <canvas id="net"></canvas>
  <script src="particle-network.js"></script>
</body>
</html>`,
    },
    { name: "particle-network.js", language: "javascript", content: JS },
  ],
  prompt:
    "请把「粒子连线背景」装进我的项目：一块全屏 canvas 背景——数十个微粒缓慢漂移，彼此距离小于阈值时连出透明度随距离衰减的细线，形成有呼吸感的网络；粒子数按面积自适应并设上限；requestAnimationFrame 驱动、resize 重建；尊重 prefers-reduced-motion（降级为静止的一帧网络）。内容层要能正常叠放阅读。先看现有页面结构，融入而不是覆盖。",
};
