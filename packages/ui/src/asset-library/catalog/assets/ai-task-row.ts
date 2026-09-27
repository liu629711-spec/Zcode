import type { AssetManifest } from "../types.js";

/**
 * AI 任务行（V2-4 首批收录，灵感来自 Beautiful UI 的 Task）：
 * 智能体执行计划的任务行列表，圆勾可点、完成划线、进度同步。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：点行切换完成态，进度文案同步。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>AI 任务行</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #cdd8ea;
  }
  .plan { width: min(420px, 92vw); }
  .plan-head { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 10px; }
  .plan-head b { font-size: 14px; }
  .plan-head span { font-size: 12px; color: #7c8db0; font-variant-numeric: tabular-nums; }
  .task {
    width: 100%; display: flex; align-items: center; gap: 12px; text-align: left;
    padding: 12px 13px; margin-top: 8px; cursor: pointer;
    border-radius: 12px; border: 1px solid rgb(148 163 184 / 0.16); background: #121826;
    color: inherit; font: inherit;
    transition: border-color 0.2s ease, background 0.2s ease;
  }
  .task:hover { border-color: rgb(148 163 184 / 0.35); }
  .task:focus-visible { outline: 2px solid #60a5fa; outline-offset: 2px; }
  .tick {
    width: 20px; height: 20px; border-radius: 50%; flex: none;
    border: 1.5px solid #46536e; display: grid; place-items: center;
    color: transparent; font-size: 11px; font-weight: 700;
    transition: background 0.2s ease, border-color 0.2s ease, color 0.2s ease;
  }
  .task.done .tick { background: linear-gradient(135deg, #34d399, #0ea5e9); border-color: transparent; color: #062018; }
  .txt b { display: block; font-size: 13.5px; font-weight: 600; transition: color 0.2s ease; }
  .txt span { font-size: 11.5px; color: #7c8db0; }
  .task.done .txt b { color: #5f6f8d; text-decoration: line-through; }
  .running { margin-left: auto; flex: none; width: 13px; height: 13px; border-radius: 50%; border: 2px solid rgb(125 211 252 / 0.25); border-top-color: #7dd3fc; animation: go 0.8s linear infinite; }
  @keyframes go { to { transform: rotate(360deg); } }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .running { animation: none; } }
</style>
</head>
<body>
  <div class="plan">
    <div class="plan-head"><b>执行计划</b><span id="progress">1/3 已完成</span></div>
    <button class="task done" type="button" data-done="true">
      <span class="tick" aria-hidden="true">✓</span>
      <span class="txt"><b>定位重复的请求封装</b><span>已完成 · 2 分钟</span></span>
    </button>
    <button class="task" type="button" data-done="false">
      <span class="tick" aria-hidden="true">✓</span>
      <span class="txt"><b>抽取共享的 fetchWithRetry</b><span>进行中 · 约需 5 分钟</span></span>
      <span class="running" aria-hidden="true"></span>
    </button>
    <button class="task" type="button" data-done="false">
      <span class="tick" aria-hidden="true">✓</span>
      <span class="txt"><b>替换三个调用方并回归</b><span>排队中</span></span>
    </button>
  </div>
  <p class="hint">点任务行切换完成态</p>
  <script>
    var tasks = Array.prototype.slice.call(document.querySelectorAll(".task"));
    var progress = document.getElementById("progress");
    function render() {
      var done = tasks.filter(function (t) { return t.dataset.done === "true"; }).length;
      progress.textContent = done + "/" + tasks.length + " 已完成";
    }
    tasks.forEach(function (t) {
      t.addEventListener("click", function () {
        t.dataset.done = t.dataset.done === "true" ? "false" : "true";
        t.classList.toggle("done", t.dataset.done === "true");
        t.querySelector(".running") && (t.querySelector(".running").style.display = t.dataset.done === "true" ? "none" : "");
        render();
      });
    });
    render();
  </script>
</body>
</html>`;

export const aiTaskRowAsset: AssetManifest = {
  id: "ai-task-row",
  title: "AI 任务行",
  titleEn: "AI Task Row",
  description: "执行计划任务行：圆勾可点、完成划线、进度同步。",
  descriptionEn: "Plan task rows with a live progress counter.",
  category: "block",
  tags: ["ai-chat", "任务清单", "进度", "beautifului", "暗色"],
  previewHtml: HTML,
  files: [{ name: "ai-task-row.html", language: "html", content: HTML }],
  prompt:
    "请把「AI 任务行」装进我的项目：智能体执行计划的任务清单组件——每组任务带「执行计划」小标与 n/m 已完成的进度计数；任务行是暗色圆角条：左侧圆形勾选位（完成后填充渐变变白勾）、中间任务名加灰色状态小字（进行中带旋转小圈），完成的整行划线降透明；行可点切换完成态并同步进度计数；整行用 button/aria-pressed 保证键盘可达；尊重 prefers-reduced-motion（转圈静止）。先看现有的清单/待办组件，融入而不是覆盖。",
  source: {
    site: "Beautiful UI",
    url: "https://beautifului.dev",
    license: "MIT",
  },
};
