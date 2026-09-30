import type { AssetManifest } from "../types.js";

/**
 * 流光扫过文字（V5 扩批，灵感来自 React Bits 的 Shiny Text）：
 * 一道高光沿文字周期性掠过，纯 CSS 背景裁剪实现。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>流光扫过文字</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 22px; }
  .shiny {
    margin: 0; font-size: clamp(26px, 5.5vw, 48px); font-weight: 800; letter-spacing: 0.05em;
    color: #39415c; /* 无光时的底色 */
    background: linear-gradient(
      110deg,
      #39415c 0%, #39415c 40%,
      #f4f7ff 50%,           /* 掠过的高光 */
      #39415c 60%, #39415c 100%
    );
    background-size: 250% 100%;
    -webkit-background-clip: text; background-clip: text;
    -webkit-text-fill-color: transparent;
    animation: sweep 2.8s linear infinite;
  }
  @keyframes sweep {
    from { background-position: 130% 0; }
    to { background-position: -80% 0; }
  }
  .badge {
    font-size: 12px; letter-spacing: 0.1em; color: #5b6b8c;
    border: 1px solid #2c3750; border-radius: 999px; padding: 5px 14px;
  }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
  @media (prefers-reduced-motion: reduce) { .shiny { animation: none; } }
</style>
</head>
<body>
  <div class="stage">
    <h1 class="shiny">限时升级 · 全新体验</h1>
    <span class="badge">适合按钮文案 / 促销标题 / 状态强调</span>
    <p class="hint">一道高光周期掠过，纯 CSS 背景裁剪</p>
  </div>
</body>
</html>`;

export const shinyTextAsset: AssetManifest = {
  id: "reactbits-shiny-text",
  title: "流光扫过文字",
  titleEn: "Shiny Text",
  description: "一道高光沿文字周期掠过，纯 CSS background-clip 实现，适合强调按钮与促销标题。",
  descriptionEn: "A light sweep periodically crossing the text via background-clip.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "高光", "渐变", "循环"],
  previewHtml: HTML,
  files: [{ name: "reactbits-shiny-text.html", language: "html", content: HTML }],
  prompt:
    "请把「流光扫过文字」装进我的项目：文字默认呈灰底色，一道高光用 linear-gradient 的亮带沿 110 度角周期性掠过（background-size 放大 + background-position 动画 + background-clip:text 透明填充）；亮带宽度与周期可调；尊重 prefers-reduced-motion（静止显示底色或常亮）。先看现有强调文案场景（按钮/徽标/促销条），融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/shiny-text",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
