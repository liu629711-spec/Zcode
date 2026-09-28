import type { AssetManifest } from "../types.js";

/**
 * AI 流式输出文本（V2-4 首批收录，灵感来自 Beautiful UI 的 Streamed Text）：
 * 回复正文按字符流式吐出，尾部方块光标，流完光标熄灭。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：「重新流式」按钮重播。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>AI 流式输出文本</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100%; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #cdd8ea;
  }
  .chat { width: min(460px, 92vw); display: flex; gap: 12px; }
  .avatar {
    width: 36px; height: 36px; border-radius: 12px; flex: none;
    display: grid; place-items: center; font-size: 13px; font-weight: 700; color: #0b101a;
    background: linear-gradient(135deg, #38bdf8, #a78bfa);
  }
  .bubble {
    flex: 1; padding: 15px 17px; border-radius: 4px 18px 18px 18px;
    background: #151c29; border: 1px solid rgb(148 163 184 / 0.16);
    font-size: 14px; line-height: 2;
  }
  .stream::after {
    content: ""; display: inline-block; width: 8px; height: 1.05em; margin-left: 3px;
    background: #7dd3fc; vertical-align: text-bottom; border-radius: 2px;
    animation: blink 0.9s steps(2) infinite;
  }
  .stream.done::after { animation: none; opacity: 0; }
  @keyframes blink { 50% { opacity: 0; } }
  .replay {
    margin-top: 14px; padding: 7px 16px; border: 1px solid #2a3550; border-radius: 8px;
    cursor: pointer; font-size: 12.5px; color: #9fb0c9; background: transparent;
  }
  .replay:hover { color: #e6edf7; border-color: #38bdf8; }
  .replay:focus-visible { outline: 2px solid #60a5fa; outline-offset: 2px; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .stream::after { animation: none; } }
</style>
</head>
<body>
  <div>
    <div class="chat">
      <span class="avatar" aria-hidden="true">AI</span>
      <div class="bubble" aria-live="polite">
        <span class="stream" id="stream"></span>
      </div>
    </div>
    <div style="text-align: center;">
      <button class="replay" type="button" id="replay">重新流式</button>
    </div>
  </div>
  <p class="hint">流完光标熄灭，可点按钮重播</p>
  <script>
    var el = document.getElementById("stream");
    var text = "好的，我先把重复的请求封装抽成 fetchWithRetry：加上指数退避与超时，再把三个调用方挨个替换，最后补一条回归测试。";
    var gen = 0;
    function stream() {
      var my = ++gen;
      el.classList.remove("done");
      el.textContent = "";
      if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
        el.textContent = text; // 降级：直接呈现全文
        el.classList.add("done");
        return;
      }
      var i = 0;
      (function tick() {
        if (my !== gen) return;
        el.textContent = text.slice(0, ++i);
        if (i < text.length) setTimeout(tick, 46);
        else el.classList.add("done");
      })();
    }
    stream();
    document.getElementById("replay").addEventListener("click", stream);
  </script>
</body>
</html>`;

export const aiStreamedTextAsset: AssetManifest = {
  id: "ai-streamed-text",
  title: "AI 流式输出文本",
  titleEn: "AI Streamed Text",
  description: "回复正文逐字流出、方块光标闪烁，流完自动熄灭。",
  descriptionEn: "Reply text streaming in behind a block caret.",
  category: "block",
  tags: ["ai-chat", "流式输出", "打字机", "beautifului", "暗色"],
  previewHtml: HTML,
  files: [{ name: "ai-streamed-text.html", language: "html", content: HTML }],
  prompt:
    "请把「AI 流式输出文本」装进我的项目：AI 聊天里的流式回复气泡——正文按字符流式追加（中文按码位，约 40~60ms/字），尾部一枚方块光标闪烁，流完光标熄灭；气泡左侧渐变头像、左上直角暗色圆角；容器 aria-live=\"polite\" 播报；流式中断/换新回复要能停掉旧的追加链；尊重 prefers-reduced-motion（直接显示全文，光标静止）。先看现有的聊天消息组件，作为消息渲染类型融入。",
  source: {
    site: "Beautiful UI",
    url: "https://beautifului.dev",
    license: "MIT",
  },
};
