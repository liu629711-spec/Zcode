import type { AssetManifest } from "../types.js";

/**
 * 磁吸按钮（自制原创）：光标靠近时按钮按指向向量小幅跟随，移开弹回。
 * 位移限制在 10px 内；触屏与 reduced-motion 直接禁用。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>磁吸按钮</title>
<style>
  body {
    min-height: 100%; margin: 0; display: grid; place-items: center;
    background: radial-gradient(110% 110% at 50% 0%, #131024 0%, #0b0a18 60%, #070610 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .zone { width: min(560px, 92vw); height: 60vh; display: grid; place-items: center; }
  .magnet {
    padding: 14px 38px; border: 0; border-radius: 14px; cursor: pointer;
    font-size: 15px; font-weight: 600; letter-spacing: 0.08em; color: #f5f3ff;
    background: linear-gradient(135deg, #8b5cf6, #ec4899);
    box-shadow: 0 10px 24px rgb(139 92 246 / 0.35);
    transition: transform 0.25s ease-out;
    will-change: transform;
  }
  .magnet:focus-visible { outline: 2px solid #c4b5fd; outline-offset: 3px; }
  @media (prefers-reduced-motion: reduce) { .magnet { transition: none; } }
</style>
</head>
<body>
  <div class="zone" id="zone">
    <button class="magnet" type="button">抓住我</button>
  </div>
  <script>
    var zone = document.getElementById("zone");
    var btn = zone.querySelector(".magnet");
    var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    var hoverable = matchMedia("(hover: hover)").matches;
    if (!reduce && hoverable) {
      zone.addEventListener("mousemove", function (e) {
        var r = btn.getBoundingClientRect();
        var dx = e.clientX - (r.left + r.width / 2);
        var dy = e.clientY - (r.top + r.height / 2);
        var dist = Math.hypot(dx, dy) || 1;
        var pull = Math.max(0, 1 - dist / 160); // 越近吸力越大
        var px = Math.max(-10, Math.min(10, dx * pull * 0.3));
        var py = Math.max(-10, Math.min(10, dy * pull * 0.3));
        btn.style.transform = "translate(" + px + "px," + py + "px)";
      });
      zone.addEventListener("mouseleave", function () { btn.style.transform = ""; });
    }
  </script>
</body>
</html>`;

export const magneticButtonAsset: AssetManifest = {
  id: "magnetic-button",
  title: "磁吸按钮",
  titleEn: "Magnetic Button",
  description: "鼠标靠近轻轻吸过去、移开弹回的磁吸按钮。",
  descriptionEn: "A button drawn toward the cursor, springing back on leave.",
  category: "control",
  tags: ["js", "按钮", "磁吸", "微交互"],
  previewHtml: HTML,
  files: [{ name: "magnetic-button.html", language: "html", content: HTML }],
  prompt:
    "请把「磁吸按钮」装进我的项目：一个鼠标靠近会被轻轻吸过去的按钮——监听按钮周围区域的 mousemove，按钮按指向光标的向量做小幅位移（距离越近吸力越大），移开后用 transition 弹回原位；位移限制在 10px 内保证不破坏布局；触屏设备（hover: none）与 prefers-reduced-motion 下直接禁用磁吸。先看现有按钮体系，融入而不是覆盖。",
};
