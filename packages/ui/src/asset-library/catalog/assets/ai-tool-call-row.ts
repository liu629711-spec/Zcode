import type { AssetManifest } from "../types.js";

/**
 * AI 工具调用行（V2-4 首批收录，灵感来自 Beautiful UI 的 Tool Call）：
 * 工具名 + 参数摘要 + 状态徽标（转圈→完成），点行展开 payload。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：点行展开/收起 payload，「重演」按钮重播转圈→完成。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>AI 工具调用行</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #cdd8ea;
  }
  .call { width: min(440px, 92vw); border-radius: 14px; background: #121826; border: 1px solid rgb(148 163 184 / 0.16); overflow: hidden; }
  .row {
    width: 100%; display: flex; align-items: center; gap: 11px;
    padding: 13px 14px; border: 0; background: transparent; cursor: pointer; text-align: left;
  }
  .row:focus-visible { outline: 2px solid #60a5fa; outline-offset: -2px; }
  .ico {
    width: 30px; height: 30px; border-radius: 9px; flex: none;
    display: grid; place-items: center; background: rgb(56 189 248 / 0.12);
  }
  .ico svg { width: 15px; height: 15px; stroke: #7dd3fc; fill: none; stroke-width: 1.8; }
  .meta { min-width: 0; }
  .meta b { display: block; font: 600 13px/1.3 ui-monospace, Consolas, monospace; color: #e6edf7; }
  .meta span { font-size: 11.5px; color: #7c8db0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; display: block; }
  .state { margin-left: auto; flex: none; display: flex; align-items: center; gap: 8px; }
  .pill { font-size: 11px; padding: 3px 9px; border-radius: 999px; color: #7dd3fc; background: rgb(125 211 252 / 0.1); border: 1px solid rgb(125 211 252 / 0.3); }
  .spin { width: 13px; height: 13px; border-radius: 50%; border: 2px solid rgb(125 211 252 / 0.25); border-top-color: #7dd3fc; animation: go 0.8s linear infinite; }
  @keyframes go { to { transform: rotate(360deg); } }
  .call.done .spin { display: none; }
  .call.done .pill { color: #6ee7b7; background: rgb(110 231 183 / 0.1); border-color: rgb(110 231 183 / 0.3); }
  .payload { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.28s ease; }
  .call.open .payload { grid-template-rows: 1fr; }
  .payload > div { overflow: hidden; }
  pre {
    margin: 2px 14px 14px; padding: 12px 14px; border-radius: 10px; overflow: auto;
    background: #0b0f18; border: 1px solid rgb(148 163 184 / 0.12);
    font: 11.5px/1.8 ui-monospace, Consolas, monospace; color: #93c5fd;
  }
  .replay { border: 0; background: transparent; color: #7c8db0; font-size: 11px; cursor: pointer; padding: 3px 6px; border-radius: 6px; }
  .replay:hover { color: #cdd8ea; background: rgb(148 163 184 / 0.1); }
  .replay:focus-visible { outline: 2px solid #60a5fa; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .spin { animation: none; border-top-color: #7dd3fc; } }
</style>
</head>
<body>
  <div class="call done" id="call">
    <button class="row" type="button" id="row" aria-expanded="false" aria-controls="payload">
      <span class="ico" aria-hidden="true">
        <svg viewBox="0 0 24 24"><path d="M4 6h16M4 12h10M4 18h7" stroke-linecap="round"/></svg>
      </span>
      <span class="meta"><b>write_file</b><span>src/store/tasks.ts · 42 行</span></span>
      <span class="state">
        <button class="replay" type="button" id="replay" title="重播执行状态">重演</button>
        <span class="spin" aria-hidden="true"></span>
        <span class="pill">已完成</span>
      </span>
    </button>
    <div class="payload" id="payload">
      <div>
        <pre>{
  "path": "src/store/tasks.ts",
  "action": "create",
  "bytes": 1268,
  "lint": "pass"
}</pre>
      </div>
    </div>
  </div>
  <p class="hint">点行展开 payload，「重演」重播执行状态</p>
  <script>
    var box = document.getElementById("call");
    var row = document.getElementById("row");
    row.addEventListener("click", function (e) {
      if (e.target.closest("#replay")) return;
      var open = box.classList.toggle("open");
      row.setAttribute("aria-expanded", String(open));
    });
    document.getElementById("replay").addEventListener("click", function () {
      box.classList.remove("done");
      setTimeout(function () { box.classList.add("done"); }, 1600);
    });
  </script>
</body>
</html>`;

export const aiToolCallRowAsset: AssetManifest = {
  id: "ai-tool-call-row",
  title: "AI 工具调用行",
  titleEn: "AI Tool Call Row",
  description: "工具名+参数摘要+状态徽标的调用行，点开看 payload。",
  descriptionEn: "A tool invocation row with status pill and payload.",
  category: "block",
  tags: ["ai-chat", "工具调用", "状态徽标", "beautifului", "暗色"],
  previewHtml: HTML,
  files: [{ name: "ai-tool-call-row.html", language: "html", content: HTML }],
  prompt:
    "请把「AI 工具调用行」装进我的项目：AI 聊天里表示一次工具调用的行组件——左侧工具图标块，中间等宽字体工具名加灰色参数摘要（超长省略），右侧状态徽标：执行中转圈（border 旋转圈）+青色「执行中」，完成后变绿色「已完成」；整行可点展开 payload（等宽 JSON 块，grid-template-rows 过渡，aria-expanded 同步）；嵌套按钮注意不要放进 button 里导致嵌套交互。尊重 prefers-reduced-motion（转圈静止）。先看现有聊天消息结构，作为消息类型融入。",
  source: {
    site: "Beautiful UI",
    url: "https://beautifului.dev",
    license: "MIT",
  },
};
