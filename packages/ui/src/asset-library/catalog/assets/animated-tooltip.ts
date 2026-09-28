import type { AssetManifest } from "../types.js";

/**
 * 灵动提示气泡（V2-4 首批收录，灵感来自 RareUI 的 Animated Tooltip）：
 * 悬停/聚焦时气泡带 overshoot 弹出，双向箭头，纯 CSS。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：两颗图标按钮，hover / 键盘 Tab 都能唤出提示。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>灵动提示气泡</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #0d1526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .bar { display: flex; gap: 26px; }
  .tip { position: relative; }
  .btn {
    width: 46px; height: 46px; border-radius: 13px; cursor: pointer;
    display: grid; place-items: center; border: 1px solid rgb(148 163 184 / 0.2);
    background: #141b28; color: #9fb0c9; transition: color 0.2s, border-color 0.2s;
  }
  .btn:hover { color: #e6edf7; border-color: #38bdf8; }
  .btn:focus-visible { outline: 2px solid #60a5fa; outline-offset: 3px; }
  .btn svg { width: 20px; height: 20px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; }
  .bubble {
    position: absolute; left: 50%; bottom: calc(100% + 12px); transform: translateX(-50%) translateY(6px) scale(0.9);
    padding: 6px 12px; border-radius: 8px; white-space: nowrap;
    background: #e6edf7; color: #0b101a; font-size: 12px; font-weight: 600;
    opacity: 0; pointer-events: none;
    transition: opacity 0.18s ease, transform 0.3s cubic-bezier(0.2, 1.6, 0.4, 1);
  }
  .bubble::after {
    content: ""; position: absolute; left: 50%; top: 100%; transform: translateX(-50%);
    border: 5px solid transparent; border-top-color: #e6edf7;
  }
  .tip:hover .bubble, .btn:focus-visible + .bubble {
    opacity: 1; transform: translateX(-50%) translateY(0) scale(1);
  }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .bubble { transition: none; } }
</style>
</head>
<body>
  <div class="bar">
    <span class="tip">
      <button class="btn" type="button" aria-label="复制代码">
        <svg viewBox="0 0 24 24"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a1 1 0 0 1 1-1h9"/></svg>
      </button>
      <span class="bubble" role="tooltip">复制代码</span>
    </span>
    <span class="tip">
      <button class="btn" type="button" aria-label="发到会话">
        <svg viewBox="0 0 24 24"><path d="M4 12h15m0 0-6-6m6 6-6 6"/></svg>
      </button>
      <span class="bubble" role="tooltip">发到会话</span>
    </span>
  </div>
  <p class="hint">悬停或用 Tab 键聚焦唤出提示</p>
</body>
</html>`;

export const animatedTooltipAsset: AssetManifest = {
  id: "animated-tooltip",
  title: "灵动提示气泡",
  titleEn: "Animated Tooltip",
  description: "overshoot 弹出的深浅反转提示气泡，键盘可达。",
  descriptionEn: "A springy inverted tooltip that pops on hover and focus.",
  category: "control",
  tags: ["css", "tooltip", "微交互", "rareui", "shadcn风"],
  previewHtml: HTML,
  files: [{ name: "animated-tooltip.html", language: "html", content: HTML }],
  prompt:
    "请把「灵动提示气泡」装进我的项目：一个图标按钮的 tooltip 组件——悬停或键盘聚焦时，浅底深字的气泡在按钮上方带 overshoot 贝塞尔弹出（translateY+scale，箭头指向按钮）；气泡用 role=\"tooltip\"、按钮带 aria-label，focus-visible 即唤出保证键盘可达；纯 CSS 实现，出弯软、收弯快；尊重 prefers-reduced-motion（瞬显瞬隐）。先看现有的图标按钮体系，融入而不是覆盖。",
  source: {
    site: "RareUI",
    url: "https://rareui.com",
    license: "MIT",
  },
};
