import type { AssetManifest } from "../types.js";

/**
 * 词组跑马循环（V5 扩批，灵感来自 React Bits 的 Text Loop）：
 * 一列词组像跑马灯一样纵向匀速循环，悬停暂停。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>词组跑马循环</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 22px; }
  .window {
    height: 5.6em; overflow: hidden; /* 只露出 2 行 */
    mask-image: linear-gradient(180deg, transparent, #000 25%, #000 75%, transparent);
    -webkit-mask-image: linear-gradient(180deg, transparent, #000 25%, #000 75%, transparent);
  }
  .reel { display: flex; flex-direction: column; gap: 0.8em; animation: scroll 9s linear infinite; }
  .window:hover .reel { animation-play-state: paused; }
  .reel span {
    font-size: clamp(20px, 4vw, 34px); font-weight: 800; letter-spacing: 0.05em;
    color: #c7d2ea; text-align: center; white-space: nowrap;
  }
  .reel span b { color: #a5b4fc; }
  @keyframes scroll {
    /* 序列复制两份，位移恰好一份高度即无缝循环 */
    to { transform: translateY(calc(-50% - 0.4em)); }
  }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
  @media (prefers-reduced-motion: reduce) { .reel { animation: none; } }
</style>
</head>
<body>
  <div class="stage">
    <div class="window">
      <div class="reel" id="reel"></div>
    </div>
    <p class="hint">悬停暂停 · 上下渐隐遮罩</p>
  </div>
  <script>
    var WORDS = ["一句话<b>生成组件</b>", "随手<b>截个想法</b>", "智能体<b>连夜排活</b>", "上线<b>只差一步</b>"];
    var reel = document.getElementById("reel");
    // 复制一份序列实现无缝循环（内容 × 2，位移 50%）
    reel.innerHTML = [...WORDS, ...WORDS].map(function (word) { return "<span>" + word + "</span>"; }).join("");
  </script>
</body>
</html>`;

export const textLoopAsset: AssetManifest = {
  id: "reactbits-text-loop",
  title: "词组跑马循环",
  titleEn: "Text Loop",
  description: "一列词组纵向匀速循环，上下渐隐遮罩收边，悬停暂停，无缝滚动。",
  descriptionEn: "A vertical marquee of phrases, masked, seamless, pause on hover.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "跑马灯", "循环", "无缝"],
  previewHtml: HTML,
  files: [{ name: "reactbits-text-loop.html", language: "html", content: HTML }],
  prompt:
    "请把「词组跑马循环」装进我的项目：一个固定高度的可视窗口（上下 linear-gradient 遮罩渐隐），内部一列词组纵向匀速滚动（CSS keyframes，序列复制两份、位移 50% 实现无缝循环），悬停暂停（animation-play-state）；词组支持内嵌高亮；尊重 prefers-reduced-motion（静止显示）。先看现有营销位/滚动标语场景，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/scroll-velocity",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
