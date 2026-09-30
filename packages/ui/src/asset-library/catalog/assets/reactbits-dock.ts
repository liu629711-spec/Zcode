import type { AssetManifest } from "../types.js";

/**
 * 指针放大坞（V5 扩批，灵感来自 React Bits 的 Dock）：
 * macOS 坞式图标条，指针靠近的图标按距离放大抬升。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>指针放大坞</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: end center; padding-bottom: 56px;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .dock {
    display: flex; align-items: flex-end; gap: 8px; padding: 10px 14px;
    border-radius: 18px; background: rgba(148, 163, 184, 0.08);
    border: 1px solid rgba(148, 163, 184, 0.16); backdrop-filter: blur(12px);
  }
  .dock .item {
    width: 46px; height: 46px; border-radius: 12px; display: grid; place-items: center;
    font-size: 20px; color: #0b0e15; transform-origin: bottom center;
    transition: transform 0.16s ease-out;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
  }
  .dock .item .tip {
    position: absolute; top: -34px; left: 50%; transform: translateX(-50%) scale(0.9);
    background: #1a2340; color: #c7d2ea; font-size: 11px; padding: 3px 8px;
    border-radius: 6px; opacity: 0; transition: opacity 0.15s; white-space: nowrap;
  }
  .dock .item:hover .tip { opacity: 1; transform: translateX(-50%) scale(1); }
</style>
</head>
<body>
  <div class="dock" id="dock" role="toolbar" aria-label="快捷入口"></div>
  <script>
    var ICONS = [
      { glyph: "🏠", tip: "首页", color: "#7dd3fc" },
      { glyph: "✉️", tip: "消息", color: "#fca5a5" },
      { glyph: "📁", tip: "项目", color: "#fcd34d" },
      { glyph: "🎨", tip: "画板", color: "#c4b5fd" },
      { glyph: "⚙️", tip: "设置", color: "#a7f3d0" },
    ];
    var dock = document.getElementById("dock");
    var items = ICONS.map(function (icon) {
      var button = document.createElement("button");
      button.className = "item";
      button.style.background = icon.color;
      button.setAttribute("aria-label", icon.tip);
      button.innerHTML = icon.glyph + '<span class="tip">' + icon.tip + "</span>";
      dock.appendChild(button);
      return button;
    });

    var BASE = 46, MAX = 78, REACH = 110; // 基准/最大尺寸/影响半径
    dock.addEventListener("pointermove", function (event) {
      items.forEach(function (item) {
        var box = item.getBoundingClientRect();
        var distance = Math.abs(event.clientX - (box.left + box.width / 2));
        var influence = Math.max(0, 1 - distance / REACH);
        var size = BASE + influence * influence * (MAX - BASE); // 平方衰减更聚焦
        item.style.width = size + "px";
        item.style.height = size + "px";
        item.style.fontSize = 20 + influence * 12 + "px";
      });
    });
    dock.addEventListener("pointerleave", function () {
      items.forEach(function (item) {
        item.style.width = BASE + "px";
        item.style.height = BASE + "px";
        item.style.fontSize = "20px";
      });
    });
  </script>
</body>
</html>`;

export const dockAsset: AssetManifest = {
  id: "reactbits-dock",
  title: "指针放大坞",
  titleEn: "Dock",
  description: "macOS 式图标坞：指针靠近的图标按距离平方放大，带悬浮提示。",
  descriptionEn: "A macOS-style dock magnifying icons by pointer proximity.",
  category: "control",
  tags: ["react-bits", "控件", "坞", "工具栏", "放大"],
  previewHtml: HTML,
  files: [{ name: "reactbits-dock.html", language: "html", content: HTML }],
  prompt:
    "请把「指针放大坞」装进我的项目：底部悬浮玻璃拟态图标条，pointermove 时按指针到各图标中心的水平距离（110px 影响半径、平方衰减）把图标从 46px 放大到最大 78px，transform-origin 底部中心保持站地感；离开复位；每图标带气泡提示与 aria-label；尺寸变化用 width/height 或 scale 都可，但过渡时长 ≤160ms 保持跟手。先看现有工具栏/快捷入口，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/components/dock",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
