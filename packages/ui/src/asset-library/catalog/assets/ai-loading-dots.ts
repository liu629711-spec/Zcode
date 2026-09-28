import type { AssetManifest } from "../types.js";

/**
 * AI 等待气泡（V2-4 首批收录，灵感来自 Beautiful UI 的 Loading）：
 * 暗色 AI 聊天里的"正在思考"气泡，三点呼吸错相。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：点气泡重播呼吸动画。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>AI 等待气泡</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100%; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .chat { display: flex; align-items: flex-end; gap: 12px; cursor: pointer; }
  .avatar {
    width: 36px; height: 36px; border-radius: 12px; flex: none;
    display: grid; place-items: center; font-size: 13px; font-weight: 700; color: #0b101a;
    background: linear-gradient(135deg, #38bdf8, #a78bfa);
  }
  .bubble {
    display: flex; align-items: center; gap: 7px;
    padding: 15px 18px; border-radius: 4px 18px 18px 18px;
    background: #151c29; border: 1px solid rgb(148 163 184 / 0.16);
  }
  .dot {
    width: 7px; height: 7px; border-radius: 50%; background: #8ea0bd;
    animation: breathe 1.2s ease-in-out infinite;
    animation-play-state: paused;
  }
  .bubble.play .dot { animation-play-state: running; }
  .bubble .dot:nth-child(2) { animation-delay: 0.18s; }
  .bubble .dot:nth-child(3) { animation-delay: 0.36s; }
  @keyframes breathe { 0%, 100% { opacity: 0.35; transform: translateY(0); } 40% { opacity: 1; transform: translateY(-4px); } }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .bubble .dot { animation: none; opacity: 0.6; } }
</style>
</head>
<body>
  <div class="chat">
    <span class="avatar" aria-hidden="true">AI</span>
    <div class="bubble play" id="bubble" role="status" aria-label="智能体正在思考">
      <span class="dot"></span><span class="dot"></span><span class="dot"></span>
    </div>
  </div>
  <p class="hint">点气泡可重播</p>
  <script>
    var bubble = document.getElementById("bubble");
    bubble.addEventListener("click", function () {
      bubble.classList.remove("play");
      void bubble.offsetWidth; // 重排重置动画
      bubble.classList.add("play");
    });
  </script>
</body>
</html>`;

export const aiLoadingDotsAsset: AssetManifest = {
  id: "ai-loading-dots",
  title: "AI 等待气泡",
  titleEn: "AI Loading Dots",
  description: "智能体回复前的等待气泡：三点呼吸错相，可点重播。",
  descriptionEn: "A thinking bubble with staggered breathing dots.",
  category: "block",
  tags: ["ai-chat", "加载态", "气泡", "beautifului", "暗色"],
  previewHtml: HTML,
  files: [{ name: "ai-loading-dots.html", language: "html", content: HTML }],
  prompt:
    "请把「AI 等待气泡」装进我的项目：AI 聊天流里智能体回复前的等待气泡——左侧方形渐变头像，右侧左上直角的暗色圆角气泡内三点呼吸（透明度+上浮，每点错相 0.18s）；点气泡可重播动画；容器带 role=\"status\" 与无障碍标签「智能体正在思考」；回复到达后气泡被真实内容替换；尊重 prefers-reduced-motion（点静止半透明）。先看现有的聊天消息组件结构，作为消息类型融入，不要另起一套。",
  source: {
    site: "Beautiful UI",
    url: "https://beautifului.dev",
    license: "MIT",
  },
};
