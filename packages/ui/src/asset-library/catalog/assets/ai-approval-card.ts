import type { AssetManifest } from "../types.js";

/**
 * AI 审批卡（V2-4 首批收录，灵感来自 Beautiful UI 的 Approval）：
 * 智能体请求执行敏感操作时的审批卡，三键决策 + 决策后状态条。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：三颗决策按钮真实可点，出结果后可「撤销」复位。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>AI 审批卡</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100%; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #cdd8ea;
  }
  .card { width: min(440px, 92vw); border-radius: 16px; background: #121826; border: 1px solid rgb(251 191 36 / 0.35); box-shadow: 0 14px 34px rgb(0 0 0 / 0.4); padding: 16px; }
  .head { display: flex; align-items: center; gap: 9px; font-weight: 600; font-size: 14px; }
  .mark { width: 22px; height: 22px; border-radius: 7px; flex: none; display: grid; place-items: center; background: rgb(251 191 36 / 0.14); color: #fbbf24; font-size: 13px; }
  .cmd {
    margin: 12px 0 0; padding: 11px 13px; border-radius: 10px;
    background: #0b0f18; border: 1px solid rgb(148 163 184 / 0.12);
    font: 12.5px/1.7 ui-monospace, Consolas, monospace; color: #fcd34d;
    white-space: pre-wrap; word-break: break-all;
  }
  .note { margin-top: 9px; font-size: 12px; color: #7c8db0; }
  .ops { display: flex; gap: 8px; margin-top: 14px; }
  .ops button {
    flex: 1; padding: 9px 0; border-radius: 9px; cursor: pointer; font-size: 13px; font-weight: 600;
    border: 1px solid rgb(148 163 184 / 0.25); background: #1a2233; color: #cdd8ea;
    transition: filter 0.15s ease;
  }
  .ops button:hover { filter: brightness(1.15); }
  .ops .allow { border-color: transparent; background: linear-gradient(135deg, #34d399, #0ea5e9); color: #062018; }
  .ops button:focus-visible { outline: 2px solid #60a5fa; outline-offset: 2px; }
  .result { display: none; align-items: center; gap: 9px; margin-top: 14px; padding: 10px 12px; border-radius: 10px; font-size: 13px; }
  .result.show { display: flex; }
  .result.ok { background: rgb(110 231 183 / 0.1); color: #6ee7b7; border: 1px solid rgb(110 231 183 / 0.3); }
  .result.no { background: rgb(248 113 113 / 0.1); color: #f87171; border: 1px solid rgb(248 113 113 / 0.3); }
  .undo { margin-left: auto; border: 0; background: transparent; color: inherit; opacity: 0.75; cursor: pointer; font-size: 12px; text-decoration: underline; }
  .undo:focus-visible { outline: 2px solid #60a5fa; outline-offset: 2px; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
</style>
</head>
<body>
  <div class="card" role="alertdialog" aria-label="智能体请求审批">
    <div class="head"><span class="mark" aria-hidden="true">!</span>智能体请求执行命令</div>
    <p class="cmd">pnpm rm @legacy/http-client &amp;&amp; pnpm install</p>
    <p class="note">该命令会移除一个依赖并重装 node_modules，影响范围已限定在 packages/ui。</p>
    <div class="ops" id="ops">
      <button type="button" class="allow" data-v="allow">仅本次允许</button>
      <button type="button" data-v="always">永久允许</button>
      <button type="button" data-v="deny">拒绝</button>
    </div>
    <div class="result" id="result">
      <span id="result-text"></span>
      <button class="undo" type="button" id="undo">撤销</button>
    </div>
  </div>
  <p class="hint">三颗按钮真实可点，决策后可撤销</p>
  <script>
    var ops = document.getElementById("ops");
    var result = document.getElementById("result");
    var text = document.getElementById("result-text");
    var words = { allow: "已允许（仅本次）· 命令开始执行", always: "已永久允许 · 后续同类命令自动放行", deny: "已拒绝 · 智能体会改用其他方案" };
    var kinds = { allow: "ok", always: "ok", deny: "no" };
    ops.addEventListener("click", function (e) {
      var btn = e.target.closest("button[data-v]");
      if (!btn) return;
      ops.style.display = "none";
      result.className = "result show " + kinds[btn.dataset.v];
      text.textContent = words[btn.dataset.v];
    });
    document.getElementById("undo").addEventListener("click", function () {
      result.className = "result";
      ops.style.display = "flex";
    });
  </script>
</body>
</html>`;

export const aiApprovalCardAsset: AssetManifest = {
  id: "ai-approval-card",
  title: "AI 审批卡",
  titleEn: "AI Approval Card",
  description: "敏感操作前的审批卡：命令原文+三键决策+可撤销结果条。",
  descriptionEn: "An approval card for sensitive agent actions.",
  category: "block",
  tags: ["ai-chat", "审批", "安全", "beautifului", "暗色"],
  previewHtml: HTML,
  files: [{ name: "ai-approval-card.html", language: "html", content: HTML }],
  prompt:
    "请把「AI 审批卡」装进我的项目：智能体请求执行敏感操作时的审批卡——琥珀色描边警示风格，含标题行（感叹标 + 「智能体请求执行命令」）、等宽字体命令原文块、一行影响范围说明，以及三颗决策按钮：仅本次允许（渐变实心）、永久允许、拒绝；决策后按钮区换成绿/红状态条（文案随决策变化）并带「撤销」链接可复位；容器 role=\"alertdialog\" 带无障碍标签，按钮保留 focus-visible。先看现有的确认弹层/权限组件，融入而不是覆盖。",
  source: {
    site: "Beautiful UI",
    url: "https://beautifului.dev",
    license: "MIT",
  },
};
