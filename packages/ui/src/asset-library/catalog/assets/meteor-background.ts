import type { AssetManifest } from "../types.js";

/**
 * 流星背景（自制原创）：canvas 星野 + 斜划流星，requestAnimationFrame 驱动。
 * 图纸分 html+js 两件可拿走；preview 是同款内联版。
 */

const JS = `const canvas = document.getElementById("sky");
const ctx = canvas.getContext("2d");
const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
let w = 0, h = 0, stars = [];
function resize() {
  w = canvas.width = innerWidth; h = canvas.height = innerHeight;
  stars = Array.from({ length: Math.floor(w / 8) }, () => ({
    x: Math.random() * w, y: Math.random() * h,
    r: Math.random() * 1.3 + 0.3, tw: Math.random() * Math.PI * 2,
  }));
}
addEventListener("resize", resize);
resize();
const meteors = [];
function spawn() {
  meteors.push({ x: w * (0.3 + Math.random() * 0.7), y: -30, len: 90 + Math.random() * 120, sp: 5 + Math.random() * 4 });
}
if (!reduceMotion) setInterval(() => { if (meteors.length < 4) spawn(); }, 1100);
function paint(t) {
  const sky = ctx.createLinearGradient(0, 0, 0, h);
  sky.addColorStop(0, "#05070f"); sky.addColorStop(0.6, "#0b1224"); sky.addColorStop(1, "#101a33");
  ctx.fillStyle = sky; ctx.fillRect(0, 0, w, h);
  for (const s of stars) {
    ctx.globalAlpha = reduceMotion ? 0.7 : Math.max(0.45 + 0.4 * Math.sin(t / 900 + s.tw), 0.15);
    ctx.fillStyle = "#dbe7ff";
    ctx.beginPath(); ctx.arc(s.x, s.y, s.r, 0, Math.PI * 2); ctx.fill();
  }
  ctx.globalAlpha = 1;
  for (let i = meteors.length - 1; i >= 0; i--) {
    const m = meteors[i];
    m.x -= m.sp * 1.4; m.y += m.sp;
    const trail = ctx.createLinearGradient(m.x, m.y, m.x + m.len, m.y - m.len * 0.72);
    trail.addColorStop(0, "rgba(186,220,255,0.95)"); trail.addColorStop(1, "rgba(186,220,255,0)");
    ctx.strokeStyle = trail; ctx.lineWidth = 2; ctx.lineCap = "round";
    ctx.beginPath(); ctx.moveTo(m.x, m.y); ctx.lineTo(m.x + m.len, m.y - m.len * 0.72); ctx.stroke();
    if (m.x < -m.len || m.y > h + m.len) meteors.splice(i, 1);
  }
}
if (reduceMotion) paint(0);
else requestAnimationFrame(function loop(t) { paint(t); requestAnimationFrame(loop); });`;

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>流星背景</title>
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; }
  canvas { display: block; width: 100%; height: 100%; }
</style>
</head>
<body>
  <canvas id="sky"></canvas>
  <script>${JS}</script>
</body>
</html>`;

export const meteorBackgroundAsset: AssetManifest = {
  id: "meteor-background",
  title: "流星背景",
  description: "canvas 星野衬底，流星偶尔斜划而过；作登录页/空态背景很出片。",
  category: "background",
  tags: ["canvas", "背景", "暗色", "氛围"],
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
  canvas { display: block; width: 100%; height: 100%; }
</style>
</head>
<body>
  <canvas id="sky"></canvas>
  <script src="meteor-background.js"></script>
</body>
</html>`,
    },
    { name: "meteor-background.js", language: "javascript", content: JS },
  ],
  prompt:
    "请把「流星背景」装进我的项目：一块全屏 canvas 星野背景——深空渐变底、细碎闪烁星点、每隔一秒多从右上往左下划过一两颗带渐隐尾迹的流星，resize 自适应；用 requestAnimationFrame 驱动，尊重 prefers-reduced-motion（降级为静态星空不动画）；内容层要能叠在它上面正常阅读。先看现有页面结构，融入而不是覆盖。",
};
