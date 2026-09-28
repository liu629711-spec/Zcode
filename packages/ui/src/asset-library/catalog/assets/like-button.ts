import type { AssetManifest } from "../types.js";

/**
 * 点赞爱心按钮（自制原创）：点击后爱心填充弹跳 + 六颗粒子迸发消散，计数同步。
 * 图纸单文件自包含，内容与 preview 同一份。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>点赞爱心按钮</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(110% 110% at 50% 0%, #141020 0%, #0d0a16 60%, #090610 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .like {
    position: relative; display: inline-flex; align-items: center; gap: 9px;
    padding: 11px 22px; border-radius: 999px; cursor: pointer;
    border: 1px solid rgb(148 163 184 / 0.25); background: #131a29;
    color: #93a3bd; font-size: 14px; font-weight: 600;
    transition: color 0.25s, border-color 0.25s, background 0.25s;
  }
  .like svg { width: 20px; height: 20px; fill: none; stroke: currentColor; stroke-width: 1.8; }
  .like.liked { color: #fb7185; border-color: rgb(251 113 133 / 0.5); background: rgb(251 113 133 / 0.08); }
  .like.liked svg { fill: #fb7185; animation: pop 0.45s cubic-bezier(0.2, 2, 0.4, 1); }
  .like:focus-visible { outline: 2px solid #fb7185; outline-offset: 3px; }
  .spark {
    position: absolute; left: 26px; top: 50%; width: 5px; height: 5px; margin: -2.5px;
    border-radius: 50%; background: #fb7185; pointer-events: none;
    animation: spark 0.6s ease-out forwards;
  }
  @keyframes pop { 0% { scale: 1; } 40% { scale: 1.35; } 70% { scale: 0.9; } 100% { scale: 1; } }
  @keyframes spark { to { transform: translate(var(--dx), var(--dy)) scale(0); opacity: 0; } }
  @media (prefers-reduced-motion: reduce) { .like svg, .spark { animation: none; } }
</style>
</head>
<body>
  <button class="like" type="button" id="like" aria-pressed="false">
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 20 C 5 14, 3 10, 5.5 6.8 C 7.6 4.2, 11 4.6, 12 7.2 C 13 4.6, 16.4 4.2, 18.5 6.8 C 21 10, 19 14, 12 20 Z" />
    </svg>
    <span>点赞</span><span id="count">128</span>
  </button>
  <script>
    var btn = document.getElementById("like");
    var count = document.getElementById("count");
    var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    var liked = false;
    btn.addEventListener("click", function () {
      liked = !liked;
      btn.classList.toggle("liked", liked);
      btn.setAttribute("aria-pressed", String(liked));
      count.textContent = String(Number(count.textContent) + (liked ? 1 : -1));
      if (reduce || !liked) return;
      for (var i = 0; i < 6; i++) {
        var spark = document.createElement("span");
        spark.className = "spark";
        var angle = (Math.PI * 2 * i) / 6 + Math.random() * 0.5;
        var dist = 22 + Math.random() * 12;
        spark.style.setProperty("--dx", Math.cos(angle) * dist + "px");
        spark.style.setProperty("--dy", Math.sin(angle) * dist + "px");
        btn.appendChild(spark);
        spark.addEventListener("animationend", function () { this.remove(); });
      }
    });
  </script>
</body>
</html>`;

export const likeButtonAsset: AssetManifest = {
  id: "like-button",
  title: "点赞爱心按钮",
  titleEn: "Like Button",
  description: "爱心弹跳加粒子迸发的点赞按钮，计数同步。",
  descriptionEn: "A like button with a pop, spark burst and live counter.",
  category: "control",
  tags: ["js", "按钮", "点赞", "微交互"],
  previewHtml: HTML,
  files: [{ name: "like-button.html", language: "html", content: HTML }],
  prompt:
    "请把「点赞爱心按钮」装进我的项目：一个带爆开粒子反馈的点赞按钮——点击后爱心填充为品牌色并做一次 overshoot 弹跳，同时从爱心中心迸出六颗小粒子向外消散，计数 +1；再点取消恢复描边态并计数 -1；aria-pressed 同步状态；尊重 prefers-reduced-motion（只变色和计数，不播爆开动画）。先看现有的互动/反馈控件风格，融入而不是覆盖。",
};
