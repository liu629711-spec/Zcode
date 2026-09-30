import type { AssetManifest } from "../types.js";

/**
 * 指针倾斜卡（V5 扩批，灵感来自 React Bits 的 Tilted Card）：
 * 3D 透视跟随指针倾斜，叠一层跟随光晕。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>指针倾斜卡</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
    perspective: 900px;
  }
  .card {
    position: relative; width: 300px; border-radius: 18px; overflow: hidden;
    background: linear-gradient(180deg, #182038, #101828);
    border: 1px solid #2c3750; padding: 22px;
    transform-style: preserve-3d; transition: transform 0.18s ease-out, box-shadow 0.3s;
    will-change: transform;
  }
  .card:hover { box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5); }
  .card .glare {
    position: absolute; inset: 0; pointer-events: none; opacity: 0; transition: opacity 0.25s;
    background: radial-gradient(220px circle at var(--gx, 50%) var(--gy, 50%), rgba(165, 180, 252, 0.22), transparent 65%);
  }
  .card:hover .glare { opacity: 1; }
  .card .thumb {
    height: 130px; border-radius: 12px; margin-bottom: 16px;
    background: linear-gradient(135deg, #6366f1 0%, #a855f7 55%, #ec4899 100%);
    transform: translateZ(28px);
  }
  .card h2 { margin: 0 0 6px; font-size: 17px; color: #e6edf5; transform: translateZ(18px); }
  .card p { margin: 0; font-size: 13px; line-height: 1.6; color: #9fb0c8; transform: translateZ(12px); }
  .hint { position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
</style>
</head>
<body>
  <div class="card" id="card">
    <div class="glare"></div>
    <div class="thumb"></div>
    <h2>倾斜视差卡</h2>
    <p>指针在哪里，卡面就朝哪里倾——缩略图与文字按深度分层浮起。</p>
  </div>
  <p class="hint">移动指针感受 3D 倾斜与分层景深</p>
  <script>
    var card = document.getElementById("card");
    var MAX_TILT = 10; // 最大倾角（度）

    card.addEventListener("pointermove", function (event) {
      var box = card.getBoundingClientRect();
      var px = (event.clientX - box.left) / box.width;   // 0~1
      var py = (event.clientY - box.top) / box.height;
      var rx = (0.5 - py) * MAX_TILT * 2;
      var ry = (px - 0.5) * MAX_TILT * 2;
      card.style.transform = "rotateX(" + rx.toFixed(2) + "deg) rotateY(" + ry.toFixed(2) + "deg)";
      card.style.setProperty("--gx", px * 100 + "%");
      card.style.setProperty("--gy", py * 100 + "%");
    });
    card.addEventListener("pointerleave", function () {
      card.style.transform = "";
    });
  </script>
</body>
</html>`;

export const tiltedCardAsset: AssetManifest = {
  id: "reactbits-tilted-card",
  title: "指针倾斜卡",
  titleEn: "Tilted Card",
  description: "3D 透视跟随指针倾斜，内部元素按深度分层浮起，叠跟随光晕。",
  descriptionEn: "Pointer-driven 3D tilt with layered parallax and a glare.",
  category: "block",
  tags: ["react-bits", "区块", "3D", "倾斜", "视差"],
  previewHtml: HTML,
  files: [{ name: "reactbits-tilted-card.html", language: "html", content: HTML }],
  prompt:
    "请把「指针倾斜卡」装进我的项目：卡片容器挂 perspective，pointermove 把指针位置映射为 rotateX/rotateY（最大 10 度、离开复位），内部缩略图/标题/正文分别 translateZ(28/18/12px) 做分层景深；叠一层 radial-gradient 光晕用 CSS 变量 --gx/--gy 跟随指针；过渡 ≤180ms 保持跟手；触屏降级为静态卡。先看现有卡片体系，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/components/tilted-card",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
