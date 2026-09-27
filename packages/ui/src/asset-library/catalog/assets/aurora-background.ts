import type { AssetManifest } from "../types.js";

/**
 * 极光背景（自制原创）：三团彩色模糊光斑以不同节奏漂移旋转，暗角收边。
 * 纯 CSS 实现，图纸单文件自包含，内容与 preview 同一份。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>极光背景</title>
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; }
  body { background: #05070f; }
  .aurora { position: fixed; inset: -15%; filter: blur(64px) saturate(1.25); }
  .blob { position: absolute; border-radius: 50%; opacity: 0.62; mix-blend-mode: screen; }
  .b1 {
    width: 55vmax; height: 32vmax; left: 5%; top: 10%;
    background: radial-gradient(closest-side, #14b8a6, transparent 72%);
    animation: drift-a 18s ease-in-out infinite alternate;
  }
  .b2 {
    width: 48vmax; height: 30vmax; right: 4%; top: 6%;
    background: radial-gradient(closest-side, #7c3aed, transparent 72%);
    animation: drift-b 23s ease-in-out infinite alternate;
  }
  .b3 {
    width: 50vmax; height: 28vmax; left: 22%; bottom: 4%;
    background: radial-gradient(closest-side, #db2777, transparent 72%);
    animation: drift-c 27s ease-in-out infinite alternate;
  }
  @keyframes drift-a { to { transform: translate(14vmax, 5vmax) rotate(24deg) scale(1.2); } }
  @keyframes drift-b { to { transform: translate(-12vmax, 8vmax) rotate(-18deg) scale(1.15); } }
  @keyframes drift-c { to { transform: translate(8vmax, -10vmax) scale(1.25); } }
  .veil {
    position: fixed; inset: 0;
    background: radial-gradient(120% 90% at 50% 10%, transparent 55%, rgb(3 5 10 / 0.82) 100%);
  }
  @media (prefers-reduced-motion: reduce) { .blob { animation: none; } }
</style>
</head>
<body>
  <div class="aurora" aria-hidden="true">
    <div class="blob b1"></div>
    <div class="blob b2"></div>
    <div class="blob b3"></div>
  </div>
  <div class="veil" aria-hidden="true"></div>
</body>
</html>`;

export const auroraBackgroundAsset: AssetManifest = {
  id: "aurora-background",
  title: "极光背景",
  titleEn: "Aurora Background",
  description: "三团彩色光斑缓慢漂移的极光氛围底，纯 CSS。",
  descriptionEn: "Drifting blurred color blobs behind a dark veil, pure CSS.",
  category: "background",
  tags: ["css", "背景", "氛围", "渐变", "暗色"],
  previewHtml: HTML,
  files: [{ name: "aurora-background.html", language: "html", content: HTML }],
  prompt:
    "请把「极光背景」装进我的项目：一块全屏氛围背景——深空底色上三团青绿/紫/品红的模糊光斑（大圆 radial-gradient + blur 滤镜 + screen 混合），各自以 18~27 秒的节奏缓慢漂移旋转，最外层加一层暗角收拢视线；纯 CSS 动画，resize 天然自适应；尊重 prefers-reduced-motion（光斑静止）；内容层叠在上面要保证对比度。先看现有页面结构，融入而不是覆盖。",
};
