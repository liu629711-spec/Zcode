import type { AssetManifest } from "../types.js";

/**
 * 掠光悬停卡（V5 扩批，灵感来自 React Bits 的 Glare Hover）：
 * 悬停时一道斜向高光从卡片上掠过。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>掠光悬停卡</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .card {
    position: relative; width: 300px; border-radius: 16px; overflow: hidden;
    background: linear-gradient(180deg, #182038, #101828); border: 1px solid #2c3750;
    padding: 22px;
  }
  .card::after {
    content: ""; position: absolute; inset: -60%; pointer-events: none;
    /* 斜向亮带，平时停在卡片外 */
    background: linear-gradient(
      to bottom right,
      transparent 42%, rgba(226, 236, 255, 0.16) 48%,
      rgba(226, 236, 255, 0.32) 50%,
      rgba(226, 236, 255, 0.16) 52%, transparent 58%
    );
    transform: translateX(-70%);
  }
  .card:hover::after { transition: transform 0.85s ease; transform: translateX(70%); }
  .card h2 { margin: 0 0 6px; font-size: 17px; color: #e6edf5; }
  .card p { margin: 0; font-size: 13px; line-height: 1.7; color: #9fb0c8; }
  .thumb { height: 120px; border-radius: 10px; margin-bottom: 14px;
    background: linear-gradient(135deg, #0ea5e9 0%, #6366f1 60%, #a855f7 100%); }
  .hint { margin: 20px 0 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; text-align: center; }
</style>
</head>
<body>
  <div>
    <div class="card">
      <div class="thumb"></div>
      <h2>掠光悬停</h2>
      <p>悬停时一道斜向高光从左上掠到右下，一次性扫过、不回放。</p>
    </div>
    <p class="hint">把鼠标移上来试试</p>
  </div>
</body>
</html>`;

export const glareHoverAsset: AssetManifest = {
  id: "reactbits-glare-hover",
  title: "掠光悬停卡",
  titleEn: "Glare Hover",
  description: "悬停时一道斜向高光一次性从卡面掠过，纯 CSS 伪元素实现。",
  descriptionEn: "A one-shot diagonal glare sweeping across the card on hover.",
  category: "block",
  tags: ["react-bits", "区块", "高光", "悬停", "卡片"],
  previewHtml: HTML,
  files: [{ name: "reactbits-glare-hover.html", language: "html", content: HTML }],
  prompt:
    "请把「掠光悬停卡」装进我的项目：卡片 ::after 伪元素铺一张 45 度 linear-gradient 斜亮带（透明→亮→透明，宽约 16%），默认 translateX(-70%) 藏在卡外，悬停时过渡到 translateX(70%) 一次性掠过（约 0.85s，不回放——移出时关 transition 让它瞬移复位）；卡片 overflow hidden 收边。先看现有可悬停卡片，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/animations/glare-hover",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
