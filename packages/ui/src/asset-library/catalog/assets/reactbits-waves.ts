import type { AssetManifest } from "../types.js";

/**
 * 层叠水波线（V5 扩批，灵感来自 React Bits 的 Waves）：
 * 数条正弦线层叠呼吸，相位与振幅各不同。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>层叠水波线</title>
<style>
  body { margin: 0; min-height: 100vh; background: #080a10; font-family: system-ui, sans-serif; }
  .hero { position: relative; min-height: 100vh; display: grid; place-items: center; overflow: hidden; }
  canvas { position: absolute; inset: 0; }
  .hero h1 {
    position: relative; margin: 0; color: #e6edf5; font-size: clamp(26px, 5vw, 46px);
    font-weight: 800; letter-spacing: 0.1em; text-align: center; pointer-events: none;
    text-shadow: 0 2px 24px rgba(8, 10, 16, 0.8);
  }
</style>
</head>
<body>
  <div class="hero">
    <canvas id="waves" aria-hidden="true"></canvas>
    <h1>屏息 · 涌动</h1>
  </div>
  <script>
    var canvas = document.getElementById("waves");
    var ctx = canvas.getContext("2d");
    var LINES = [
      { amp: 26, period: 320, speed: 0.018, color: "rgba(99, 102, 241, 0.5)", width: 2 },
      { amp: 18, period: 220, speed: 0.012, color: "rgba(34, 211, 238, 0.35)", width: 1.5 },
      { amp: 34, period: 420, speed: 0.009, color: "rgba(139, 92, 246, 0.25)", width: 1 },
    ];

    function resize() {
      canvas.width = canvas.offsetWidth * devicePixelRatio;
      canvas.height = canvas.offsetHeight * devicePixelRatio;
      ctx.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0);
    }
    resize();
    window.addEventListener("resize", resize);

    var t = 0;
    function draw() {
      t += 1;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      var w = canvas.offsetWidth;
      var h = canvas.offsetHeight;
      LINES.forEach(function (line, index) {
        var baseY = h * (0.38 + index * 0.12);
        ctx.beginPath();
        for (var x = -20; x <= w + 20; x += 6) {
          // 双正弦叠加 + 缓慢漂移相位，避免机械重复
          var y = baseY
            + Math.sin(x / line.period + t * line.speed) * line.amp
            + Math.sin(x / (line.period * 0.37) - t * line.speed * 0.6) * line.amp * 0.3;
          if (x === -20) ctx.moveTo(x, y); else ctx.lineTo(x, y);
        }
        ctx.strokeStyle = line.color;
        ctx.lineWidth = line.width;
        ctx.stroke();
      });
      requestAnimationFrame(draw);
    }
    draw();
  </script>
</body>
</html>`;

export const wavesAsset: AssetManifest = {
  id: "reactbits-waves",
  title: "层叠水波线",
  titleEn: "Waves",
  description: "数条双正弦叠加的水波线层叠呼吸，相位错开，安静但有生命力。",
  descriptionEn: "Layered dual-sine wave lines breathing out of phase.",
  category: "background",
  tags: ["react-bits", "背景", "水波", "canvas", "线条"],
  previewHtml: HTML,
  files: [{ name: "reactbits-waves.html", language: "html", content: HTML }],
  prompt:
    "请把「层叠水波线」装进我的项目：hero 背景一层全屏 canvas，画 3 条左右相位、振幅、周期、透明度各异的曲线——每条用双正弦叠加（主波 + 0.37 倍周期的副波×0.3）避免机械重复，随时间缓慢漂移；devicePixelRatio 适配；前景文字不挡交互。先看现有 hero 背景层，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/backgrounds/waves",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
