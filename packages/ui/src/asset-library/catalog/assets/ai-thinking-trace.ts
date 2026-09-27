import type { AssetManifest } from "../types.js";

/**
 * AI 思考过程折叠卡（V2-4 首批收录，灵感来自 Beautiful UI 的 Thinking）：
 * "思考中…" shimmer 行可展开为逐步推理列表，最后一步流光渐隐。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：点头部折叠/展开。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>AI 思考过程</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #cdd8ea;
  }
  .think { width: min(420px, 90vw); border-radius: 16px; background: #121826; border: 1px solid rgb(148 163 184 / 0.16); overflow: hidden; }
  .head {
    width: 100%; display: flex; align-items: center; gap: 10px;
    padding: 14px 16px; border: 0; background: transparent; cursor: pointer;
    color: #cdd8ea; font: 600 13.5px/1 system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .head:focus-visible { outline: 2px solid #60a5fa; outline-offset: -2px; }
  .pulse { width: 8px; height: 8px; border-radius: 50%; background: #38bdf8; animation: pulse 1.6s ease-in-out infinite; flex: none; }
  @keyframes pulse { 0%, 100% { box-shadow: 0 0 0 0 rgb(56 189 248 / 0.5); } 50% { box-shadow: 0 0 0 7px rgb(56 189 248 / 0); } }
  .chev { margin-left: auto; transition: transform 0.25s ease; color: #7c8db0; font-size: 11px; }
  .think.open .chev { transform: rotate(180deg); }
  .body { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.3s ease; }
  .think.open .body { grid-template-rows: 1fr; }
  .body > div { overflow: hidden; }
  .steps { padding: 2px 18px 16px; font: 12.5px/2 ui-monospace, Consolas, monospace; color: #8ea0bd; }
  .steps li { list-style: none; padding-left: 18px; position: relative; }
  .steps li::before { content: "›"; position: absolute; left: 2px; color: #38bdf8; }
  .now {
    color: transparent; background: linear-gradient(90deg, #8ea0bd 20%, #e0f2fe 45%, #8ea0bd 70%);
    background-size: 200% 100%; -webkit-background-clip: text; background-clip: text;
    animation: shine 1.8s linear infinite;
  }
  @keyframes shine { to { background-position: -200% 0; } }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .pulse, .now { animation: none; }
    .now { color: #e0f2fe; background: none; }
    .body, .chev { transition: none; }
  }
</style>
</head>
<body>
  <div class="think open" id="think">
    <button class="head" type="button" id="head" aria-expanded="true" aria-controls="steps">
      <span class="pulse" aria-hidden="true"></span>思考过程 · 5 步
      <span class="chev" aria-hidden="true">▼</span>
    </button>
    <div class="body" id="steps">
      <div>
        <ol class="steps">
          <li>读取当前路由与状态管理结构</li>
          <li>比对现有 store 切分方式</li>
          <li>确定改动范围：2 个文件</li>
          <li>拟写迁移步骤与回滚点</li>
          <li class="now">正在推理最优接法…</li>
        </ol>
      </div>
    </div>
  </div>
  <p class="hint">点头部可折叠 / 展开</p>
  <script>
    var box = document.getElementById("think");
    var head = document.getElementById("head");
    head.addEventListener("click", function () {
      var open = box.classList.toggle("open");
      head.setAttribute("aria-expanded", String(open));
    });
  </script>
</body>
</html>`;

export const aiThinkingTraceAsset: AssetManifest = {
  id: "ai-thinking-trace",
  title: "AI 思考过程折叠卡",
  titleEn: "AI Thinking Trace",
  description: "可展开的推理痕迹：步骤等宽列表，末行流光滚动。",
  descriptionEn: "A collapsible reasoning trace with a shimmering tail.",
  category: "block",
  tags: ["ai-chat", "思考过程", "折叠面板", "beautifului", "暗色"],
  previewHtml: HTML,
  files: [{ name: "ai-thinking-trace.html", language: "html", content: HTML }],
  prompt:
    "请把「AI 思考过程折叠卡」装进我的项目：AI 聊天里的推理痕迹组件——头部一行带呼吸光点的「思考过程 · N 步」可点折叠/展开（aria-expanded 同步），展开后是等宽字体的步骤列表，每行前缀 › 号，最后一行用 background-clip:text 的流光渐隐表示进行中；折叠动画用 grid-template-rows 0fr→1fr 过渡；完成态光点转灰色对勾、流光行定格为普通文字；尊重 prefers-reduced-motion（无 shimmer 无呼吸）。先看现有聊天消息结构，作为消息类型融入。",
  source: {
    site: "Beautiful UI",
    url: "https://beautifului.dev",
    license: "MIT",
  },
};
