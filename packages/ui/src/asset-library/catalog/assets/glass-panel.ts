import type { AssetManifest } from "../types.js";

/**
 * 玻璃拟态数据面板（自制原创）：backdrop-filter 毛玻璃卡衬彩色光斑，
 * 三列指标展示。图纸单文件自包含，内容与 preview 同一份。
 * V2-3 交互：三个指标数字 contenteditable，点数字直接改。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>玻璃拟态数据面板</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100%; display: grid; place-items: center; overflow: hidden;
    background: #0a0f1c;
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .blob { position: absolute; border-radius: 50%; filter: blur(58px); opacity: 0.7; }
  .g1 { width: 300px; height: 300px; left: 14%; top: 18%; background: #2563eb; }
  .g2 { width: 260px; height: 260px; right: 12%; top: 40%; background: #9333ea; }
  .g3 { width: 240px; height: 240px; left: 34%; bottom: 6%; background: #0d9488; }
  .glass {
    position: relative; width: min(380px, 88vw); padding: 24px 26px; border-radius: 20px;
    background: rgb(255 255 255 / 0.07);
    border: 1px solid rgb(255 255 255 / 0.16);
    backdrop-filter: blur(18px) saturate(1.3); -webkit-backdrop-filter: blur(18px) saturate(1.3);
    box-shadow: 0 20px 50px rgb(0 0 0 / 0.4);
    color: #e7edf7;
  }
  .glass header { display: flex; align-items: center; justify-content: space-between; }
  .glass h2 { font-size: 16px; font-weight: 600; }
  .badge {
    padding: 3px 10px; border-radius: 999px; font-size: 11px; letter-spacing: 0.12em;
    color: #6ee7b7; background: rgb(110 231 183 / 0.12); border: 1px solid rgb(110 231 183 / 0.3);
  }
  .stats { display: flex; margin-top: 20px; }
  .stats > div { flex: 1; padding: 2px 14px; }
  .stats > div + div { border-left: 1px solid rgb(255 255 255 / 0.12); }
  .stats b { display: block; font-size: 24px; font-variant-numeric: tabular-nums; }
  .stats b[contenteditable] { outline: none; cursor: text; border-radius: 4px; }
  .stats b[contenteditable]:focus { background: rgb(125 211 252 / 0.12); }
  .stats span { font-size: 12px; color: #9fb0c9; }
  .hint {
    position: fixed; left: 50%; bottom: 16px; transform: translateX(-50%);
    margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
  @media (prefers-reduced-motion: no-preference) {
    .glass { animation: rise 0.6s ease-out; }
    @keyframes rise { from { opacity: 0; transform: translateY(14px); } }
  }
</style>
</head>
<body>
  <div class="blob g1" aria-hidden="true"></div>
  <div class="blob g2" aria-hidden="true"></div>
  <div class="blob g3" aria-hidden="true"></div>
  <section class="glass" aria-label="本周概览">
    <header><h2>本周概览</h2><span class="badge">LIVE</span></header>
    <div class="stats">
      <div><b contenteditable="true" spellcheck="false">2,847</b><span>访问</span></div>
      <div><b contenteditable="true" spellcheck="false">96.4%</b><span>留存</span></div>
      <div><b contenteditable="true" spellcheck="false">+18%</b><span>转化</span></div>
    </div>
  </section>
  <p class="hint">点数字可直接编辑</p>
</body>
</html>`;

export const glassPanelAsset: AssetManifest = {
  id: "glass-panel",
  title: "玻璃拟态数据面板",
  titleEn: "Glass Stats Panel",
  description: "毛玻璃指标卡：高光描边衬彩色光斑，三列数据。",
  descriptionEn: "A frosted stats card floating over blurred color blobs.",
  category: "block",
  tags: ["css", "卡片", "毛玻璃", "数据"],
  previewHtml: HTML,
  files: [{ name: "glass-panel.html", language: "html", content: HTML }],
  prompt:
    "请把「玻璃拟态数据面板」装进我的项目：一个 backdrop-filter 毛玻璃信息卡——半透明白 7% 底、1px 高光描边、blur(18px)，背后衬几团彩色光斑，卡内放标题行和三列关键指标（数值 + 标签，中间细分隔线），入场一次轻微上浮；深浅底都要能衬住。先看现有卡片/面板体系，作为数据展示变体融入，不要另起一套玻璃风格的全局变量。",
};
