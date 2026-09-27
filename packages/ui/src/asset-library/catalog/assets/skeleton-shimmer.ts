import type { AssetManifest } from "../types.js";

/**
 * 骨架屏 shimmer（自制原创）：加载占位卡片，高光自左向右扫过。
 * 图纸单文件自包含，内容与 preview 同一份。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>骨架屏 shimmer</title>
<style>
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: #0c111b;
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .card {
    width: min(340px, 88vw); padding: 20px; border-radius: 16px;
    background: #121a29; border: 1px solid rgb(148 163 184 / 0.12);
    display: grid; grid-template-columns: 44px 1fr; gap: 12px 14px; align-items: center;
  }
  .bone {
    border-radius: 8px;
    background: linear-gradient(100deg, #1a2436 40%, #26334c 50%, #1a2436 60%) #1a2436;
    background-size: 240% 100%;
    animation: shimmer 1.6s linear infinite;
  }
  .avatar { grid-row: span 2; width: 44px; height: 44px; border-radius: 50%; }
  .line { height: 12px; }
  .line.short { width: 58%; }
  @keyframes shimmer { from { background-position: 120% 0; } to { background-position: -120% 0; } }
  @media (prefers-reduced-motion: reduce) {
    .bone { animation: none; background: #22304a; }
  }
</style>
</head>
<body>
  <div class="card" role="status" aria-label="内容加载中">
    <div class="bone avatar"></div>
    <div class="bone line"></div>
    <div class="bone line short"></div>
  </div>
</body>
</html>`;

export const skeletonShimmerAsset: AssetManifest = {
  id: "skeleton-shimmer",
  title: "骨架屏 shimmer",
  description: "加载占位卡片：灰骨上加一道自左向右扫过的高光。",
  category: "block",
  tags: ["骨架屏", "加载态", "css", "暗色"],
  previewHtml: HTML,
  files: [{ name: "skeleton-shimmer.html", language: "html", content: HTML }],
  prompt:
    "请把「骨架屏 shimmer」装进我的项目：列表/卡片加载时的占位骨架——按真实内容布局摆圆形头像位与两行文本条，灰骨底上加一道自左向右循环扫过的高光（background-position 线性动画）；占位骨架要带 role=\"status\" 与无障碍标签；尊重 prefers-reduced-motion（降级为静态灰骨）。先看现有的加载态与卡片样式，融入而不是覆盖。",
};
