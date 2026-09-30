import type { AssetManifest } from "../types.js";

/**
 * 故障字符雨（V5 扩批，灵感来自 React Bits 的 Letter Glitch）：
 * 全屏网格随机字符高频闪灭，三色故障调。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>故障字符雨</title>
<style>
  body { margin: 0; min-height: 100vh; background: #07090f; font-family: ui-monospace, Menlo, Consolas, monospace; overflow: hidden; }
  canvas { display: block; width: 100vw; height: 100vh; }
</style>
</head>
<body>
  <canvas id="glitch" aria-hidden="true"></canvas>
  <script>
    var canvas = document.getElementById("glitch");
    var ctx = canvas.getContext("2d");
    var GLYPHS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#%&@$*+=<>[]{}";
    var COLORS = ["#8b5cf6", "#22d3ee", "#f472b6"]; // 故障三色
    var CELL = 24;

    var grid = [];
    function resize() {
      canvas.width = innerWidth;
      canvas.height = innerHeight;
      grid = [];
      for (var y = CELL; y < canvas.height; y += CELL) {
        for (var x = CELL; x < canvas.width; x += CELL) {
          grid.push({ x: x, y: y, glyph: pick(GLYPHS), color: pick(COLORS), life: 0 });
        }
      }
    }
    function pick(list) { return list[Math.floor(Math.random() * list.length)]; }

    resize();
    window.addEventListener("resize", resize);

    function draw() {
      ctx.fillStyle = "rgba(7, 9, 15, 0.35)"; // 拖影底
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      // 每帧随机点亮少量格子，其余自然衰减
      for (var i = 0; i < grid.length * 0.008 + 2; i++) {
        var cell = grid[Math.floor(Math.random() * grid.length)];
        cell.glyph = pick(GLYPHS);
        cell.color = pick(COLORS);
        cell.life = 1;
      }
      grid.forEach(function (cell) {
        if (cell.life <= 0.02) return;
        ctx.globalAlpha = cell.life;
        ctx.fillStyle = cell.color;
        ctx.font = "600 13px ui-monospace, Menlo, Consolas, monospace";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText(cell.glyph, cell.x, cell.y);
        cell.life *= 0.72; // 闪灭衰减
      });
      ctx.globalAlpha = 1;
      requestAnimationFrame(draw);
    }
    draw();
  </script>
</body>
</html>`;

export const letterGlitchAsset: AssetManifest = {
  id: "reactbits-letter-glitch",
  title: "故障字符雨",
  titleEn: "Letter Glitch",
  description: "全屏字符网格随机闪灭，紫青粉三色故障调，带拖影余韵。",
  descriptionEn: "A grid of glyphs flickering in glitchy tri-color with trails.",
  category: "background",
  tags: ["react-bits", "背景", "故障", "字符", "赛博"],
  previewHtml: HTML,
  files: [{ name: "reactbits-letter-glitch.html", language: "html", content: HTML }],
  prompt:
    "请把「故障字符雨」装进我的项目：全屏 canvas 划成约 24px 的字符网格，每帧随机点亮极少量格子（随机字符 + 紫/青/粉三色），点亮的字符按 0.72 系数逐帧衰减形成闪灭；每帧先用半透明底色覆盖制造拖影；窗口 resize 重建网格。作为区块背景层，前景内容不被遮挡（aria-hidden、pointer-events 穿透）。先看现有背景层结构，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/backgrounds/letter-glitch",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
