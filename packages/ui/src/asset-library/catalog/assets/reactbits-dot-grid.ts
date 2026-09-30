import type { AssetManifest } from "../types.js";

/**
 * 指针点阵波纹（V5 扩批，灵感来自 React Bits 的 Dot Grid）：
 * canvas 点阵随指针靠近而放大发亮，像水面被搅动。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>指针点阵波纹</title>
<style>
  body { margin: 0; min-height: 100vh; background: #080a10; font-family: system-ui, sans-serif; }
  .hero { position: relative; min-height: 100vh; display: grid; place-items: center; overflow: hidden; }
  canvas { position: absolute; inset: 0; }
  .hero h1 {
    position: relative; margin: 0; color: #e6edf5; font-size: clamp(26px, 5vw, 46px);
    font-weight: 800; letter-spacing: 0.08em; text-align: center;
    text-shadow: 0 2px 24px rgba(8, 10, 16, 0.8); pointer-events: none;
  }
</style>
</head>
<body>
  <div class="hero">
    <canvas id="dots" aria-hidden="true"></canvas>
    <h1>鼠标划过点阵</h1>
  </div>
  <script>
    var canvas = document.getElementById("dots");
    var ctx = canvas.getContext("2d");
    var pointer = { x: -9999, y: -9999 };
    var GAP = 26; // 点距

    function resize() {
      canvas.width = canvas.offsetWidth * devicePixelRatio;
      canvas.height = canvas.offsetHeight * devicePixelRatio;
      ctx.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0);
    }
    resize();
    window.addEventListener("resize", resize);

    document.querySelector(".hero").addEventListener("pointermove", function (event) {
      var box = canvas.getBoundingClientRect();
      pointer.x = event.clientX - box.left;
      pointer.y = event.clientY - box.top;
    });
    document.querySelector(".hero").addEventListener("pointerleave", function () {
      pointer.x = pointer.y = -9999;
    });

    function draw() {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      for (var x = GAP / 2; x < canvas.offsetWidth; x += GAP) {
        for (var y = GAP / 2; y < canvas.offsetHeight; y += GAP) {
          var dx = x - pointer.x;
          var dy = y - pointer.y;
          var distance = Math.sqrt(dx * dx + dy * dy);
          var influence = Math.max(0, 1 - distance / 160); // 影响半径 160px
          var radius = 1.1 + influence * 2.2;
          var alpha = 0.16 + influence * 0.7;
          ctx.beginPath();
          ctx.arc(x, y, radius, 0, Math.PI * 2);
          ctx.fillStyle = "rgba(" + (122 + Math.round(influence * 100)) + ", " + (140 + Math.round(influence * 60)) + ", 246, " + alpha.toFixed(3) + ")";
          ctx.fill();
        }
      }
      requestAnimationFrame(draw);
    }
    draw();
  </script>
</body>
</html>`;

export const dotGridAsset: AssetManifest = {
  id: "reactbits-dot-grid",
  title: "指针点阵波纹",
  titleEn: "Dot Grid",
  description: "canvas 点阵背景，指针靠近的点放大发亮，像水面被搅开的涟漪。",
  descriptionEn: "A canvas dot grid lighting up around the pointer like ripples.",
  category: "background",
  tags: ["react-bits", "背景", "点阵", "指针", "canvas"],
  previewHtml: HTML,
  files: [{ name: "reactbits-dot-grid.html", language: "html", content: HTML }],
  prompt:
    "请把「指针点阵波纹」装进我的项目：hero 背景一层全屏 canvas 点阵（间距约 26px），requestAnimationFrame 里按指针距离计算影响系数（半径 160px 内线性衰减），让靠近的点半径变大、透明度升高并向品牌色偏移；指针离开回填到基态；canvas 尺寸随容器与 devicePixelRatio 缩放；pointer-events 不挡前景内容。先看现有 hero 背景，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/backgrounds/dot-grid",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
