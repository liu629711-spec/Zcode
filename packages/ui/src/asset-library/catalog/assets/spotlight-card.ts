import type { AssetManifest } from "../types.js";

/**
 * 聚光卡片（V2-4 首批收录，灵感来自 RareUI 的 Spotlight）：
 * 一圈径向渐变光斑贴着光标在卡内游走，边框也随之点亮。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：光标在卡上移动即驱动光斑（mousemove 写 CSS 变量）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>聚光卡片</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100%; display: grid; place-items: center;
    background: #0a0d15;
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .spot {
    --mx: 50%; --my: 50%;
    position: relative; width: min(420px, 90vw); border-radius: 18px; padding: 1.5px;
    background: #1a2233;
  }
  .spot::before {
    content: ""; position: absolute; inset: 0; border-radius: inherit;
    background: radial-gradient(220px circle at var(--mx) var(--my), rgb(56 189 248 / 0.9), transparent 65%);
    opacity: 0; transition: opacity 0.3s ease;
  }
  .spot:hover::before { opacity: 1; }
  .inner {
    position: relative; border-radius: 16.5px; padding: 26px 24px;
    background: #0e1320;
  }
  .inner::after {
    content: ""; position: absolute; inset: 0; border-radius: inherit; pointer-events: none;
    background: radial-gradient(260px circle at var(--mx) var(--my), rgb(56 189 248 / 0.08), transparent 60%);
    opacity: 0; transition: opacity 0.3s ease;
  }
  .spot:hover .inner::after { opacity: 1; }
  .tag { display: inline-block; padding: 3px 10px; border-radius: 999px; font-size: 11px; letter-spacing: 0.1em; color: #7dd3fc; background: rgb(125 211 252 / 0.1); }
  h3 { margin: 13px 0 8px; font-size: 19px; color: #e6edf7; }
  p { font-size: 13.5px; line-height: 1.85; color: #9fb0c9; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #3d4a66; user-select: none;
  }
</style>
</head>
<body>
  <div class="spot" id="spot">
    <div class="inner">
      <span class="tag">PRO TIP</span>
      <h3>光斑跟着光标走</h3>
      <p>卡片的边框与内层各铺一层 radial-gradient，圆心由 --mx/--my 两个 CSS 变量驱动；mousemove 只写变量，渲染交给 CSS，悬停渐显、移开渐隐。</p>
    </div>
  </div>
  <p class="hint">把光标移到卡片上</p>
  <script>
    var spot = document.getElementById("spot");
    spot.addEventListener("pointermove", function (e) {
      var r = spot.getBoundingClientRect();
      spot.style.setProperty("--mx", (e.clientX - r.left) + "px");
      spot.style.setProperty("--my", (e.clientY - r.top) + "px");
    });
  </script>
</body>
</html>`;

export const spotlightCardAsset: AssetManifest = {
  id: "spotlight-card",
  title: "聚光卡片",
  titleEn: "Spotlight Card",
  description: "边框与内层光斑贴着光标游走的暗色卡片。",
  descriptionEn: "A card whose border glow follows the cursor.",
  category: "block",
  tags: ["css", "卡片", "光效", "rareui", "shadcn风"],
  previewHtml: HTML,
  files: [{ name: "spotlight-card.html", language: "html", content: HTML }],
  prompt:
    "请把「聚光卡片」装进我的项目：一个光标聚光卡片——外层 1.5px 深色描边底上叠一层 radial-gradient 光斑（220px 圆），内层背景再叠一层更柔的光，两层圆心都由 --mx/--my CSS 变量驱动，pointermove 时把光标相对坐标写进变量，渲染全在 CSS 完成；悬停渐显、移开渐隐；不引第三方库、不逐帧改 DOM。先看现有卡片体系，作为强调变体融入，不要覆盖全局卡片样式。",
  source: {
    site: "RareUI",
    url: "https://rareui.com",
    license: "MIT",
  },
};
