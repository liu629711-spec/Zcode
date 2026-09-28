import type { AssetManifest } from "../types.js";

/**
 * 动感滚动列表（V2-4 首批收录，灵感来自 RareUI 的 Animated List）：
 * 动态流（活动/通知）逐条滑入，首条到顶后沉底循环；悬停暂停，点击点亮。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：悬停暂停轮换，点击行点亮高亮。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>动感滚动列表</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #cdd8ea;
  }
  .list { width: min(380px, 90vw); display: grid; gap: 9px; }
  .item {
    display: flex; align-items: center; gap: 11px;
    padding: 12px 14px; border-radius: 13px; cursor: pointer;
    background: #121826; border: 1px solid rgb(148 163 184 / 0.16);
    animation: slide-in 0.5s cubic-bezier(0.2, 0.9, 0.3, 1.1);
  }
  .item.lit { border-color: #38bdf8; background: rgb(56 189 248 / 0.08); }
  @keyframes slide-in { from { opacity: 0; transform: translateY(-10px) scale(0.97); } }
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .item b { font-size: 13px; color: #e6edf7; font-weight: 600; }
  .item time { margin-left: auto; font-size: 11px; color: #7c8db0; font-variant-numeric: tabular-nums; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .item { animation: none; } }
</style>
</head>
<body>
  <div>
    <div class="list" id="list" aria-label="团队动态">
      <div class="item"><span class="dot" style="background:#34d399"></span><b>小雨 合并了 PR #128</b><time>刚刚</time></div>
      <div class="item"><span class="dot" style="background:#38bdf8"></span><b>阿哲 发布了 v0.9.2</b><time>2 分钟前</time></div>
      <div class="item"><span class="dot" style="background:#fbbf24"></span><b>CI 流水线转绿</b><time>5 分钟前</time></div>
      <div class="item"><span class="dot" style="background:#f472b6"></span><b>叶子 认领了新 issue</b><time>9 分钟前</time></div>
    </div>
  </div>
  <p class="hint">悬停暂停轮换，点行点亮</p>
  <script>
    var box = document.getElementById("list");
    var timer = null;
    function cycle() {
      var first = box.firstElementChild;
      first.style.animation = "none";
      box.appendChild(first);
      void first.offsetWidth; // 重排后重放进场动画
      first.style.animation = "";
    }
    function play() { if (!timer) timer = setInterval(cycle, 2200); }
    function pause() { clearInterval(timer); timer = null; }
    play();
    box.addEventListener("mouseenter", pause);
    box.addEventListener("mouseleave", play);
    box.addEventListener("click", function (e) {
      var item = e.target.closest(".item");
      if (item) item.classList.toggle("lit");
    });
  </script>
</body>
</html>`;

export const animatedListAsset: AssetManifest = {
  id: "animated-list",
  title: "动感滚动列表",
  titleEn: "Animated List",
  description: "动态流逐条滑入循环轮换，悬停暂停、点击点亮。",
  descriptionEn: "A cycling activity feed that pauses on hover.",
  category: "block",
  tags: ["js", "动态流", "列表", "rareui", "shadcn风"],
  previewHtml: HTML,
  files: [{ name: "animated-list.html", language: "html", content: HTML }],
  prompt:
    "请把「动感滚动列表」装进我的项目：一个团队动态/通知轮换流——每条彩点+标题+时间，新条目从顶部带轻微下落缩放滑入，最旧的沉到队尾循环（DOM 移位后重放动画）；悬停暂停轮换、移开继续；条目可点点亮表示已读；尊重 prefers-reduced-motion（停止轮换，静态展示）。条数固定用复用节点，不要无限 append。先看现有的通知/动态场景，融入而不是覆盖。",
  source: {
    site: "RareUI",
    url: "https://rareui.com",
    license: "MIT",
  },
};
