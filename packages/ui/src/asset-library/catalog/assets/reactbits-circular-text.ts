import type { AssetManifest } from "../types.js";

/**
 * 环形旋转文字（V5 扩批，灵感来自 React Bits 的 Circular Text）：
 * 文字沿圆周排布并整体慢速旋转，中心可放图标或徽标。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>环形旋转文字</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 20px; }
  .orbit { position: relative; width: 220px; height: 220px; }
  .orbit .ring {
    position: absolute; inset: 0;
    animation: spin 14s linear infinite;
  }
  .orbit .ring span {
    position: absolute; left: 50%; top: 50%;
    font-size: 13px; font-weight: 600; letter-spacing: 0.18em; color: #9fb0c8;
    transform-origin: 0 0; /* 先平移到圆周，再旋转到对应角度 */
  }
  .orbit .core {
    position: absolute; inset: 0; margin: auto; width: 92px; height: 92px;
    border-radius: 50%; display: grid; place-items: center;
    background: linear-gradient(180deg, #1a2340, #101830);
    border: 1px solid #2c3750; color: #a5b4fc; font-weight: 800; font-size: 20px;
    box-shadow: 0 0 40px rgba(99, 102, 241, 0.25);
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
  @media (prefers-reduced-motion: reduce) { .orbit .ring { animation: none; } }
</style>
</head>
<body>
  <div class="stage">
    <div class="orbit" id="orbit" aria-label="SCROLL TO EXPLORE · 向下滚动探索">
      <div class="ring" id="ring" aria-hidden="true"></div>
      <div class="core">GO</div>
    </div>
    <p class="hint">文字沿圆周排布并整体慢速旋转</p>
  </div>
  <script>
    var ring = document.getElementById("ring");
    var text = " · SCROLL TO EXPLORE · 向下滚动探索 ";
    var radius = 88; // 圆周半径（px），与容器 220px 匹配
    Array.from(text).forEach(function (ch, index, list) {
      var span = document.createElement("span");
      span.textContent = ch;
      var angle = (index / list.length) * 360;
      span.style.transform = "rotate(" + angle + "deg) translate(" + radius + "px)";
      ring.appendChild(span);
    });
  </script>
</body>
</html>`;

export const circularTextAsset: AssetManifest = {
  id: "reactbits-circular-text",
  title: "环形旋转文字",
  titleEn: "Circular Text",
  description: "文字沿圆周逐字排布并整体慢速旋转，中心放徽标，适合滚动引导徽章。",
  descriptionEn: "Glyphs laid out on a circle, slowly orbiting a center badge.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "圆周", "徽章", "旋转"],
  previewHtml: HTML,
  files: [{ name: "reactbits-circular-text.html", language: "html", content: HTML }],
  prompt:
    "请把「环形旋转文字」装进我的项目：一段引导文案逐字沿圆周排布（每个字 rotate(角度) translate(半径) 定位在圆周上），整圈以 14s/圈匀速旋转，中心放一个圆形徽标（图标或短词）；半径与字号随容器尺寸缩放，悬停可暂停旋转；尊重 prefers-reduced-motion（静止）。先看现有 CTA/滚动引导徽章场景，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/circular-text",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
