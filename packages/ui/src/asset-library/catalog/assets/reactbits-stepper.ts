import type { AssetManifest } from "../types.js";

/**
 * 分步进度（V5 扩批，灵感来自 React Bits 的 Stepper）：
 * 步骤圆点 + 连接线填充推进，可点击回跳。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>分步进度</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 26px; width: min(560px, 92vw); }
  .stepper { display: flex; align-items: center; width: 100%; }
  .node { display: grid; place-items: center; gap: 8px; flex: 0 0 auto; }
  .node .dot {
    width: 34px; height: 34px; border-radius: 50%; display: grid; place-items: center;
    background: #141b2e; border: 2px solid #2c3750; color: #64748b;
    font-size: 13px; font-weight: 700; cursor: pointer; transition: all 0.25s ease;
  }
  .node.done .dot { background: #6366f1; border-color: #6366f1; color: #fff; }
  .node.current .dot { border-color: #a5b4fc; color: #c7d2ea; box-shadow: 0 0 0 5px rgba(99, 102, 241, 0.18); }
  .node .label { font-size: 12px; color: #64748b; white-space: nowrap; }
  .node.done .label, .node.current .label { color: #c7d2ea; }
  .track { flex: 1 1 auto; height: 3px; margin: 0 8px 22px; background: #1c2438; border-radius: 999px; overflow: hidden; }
  .track i { display: block; height: 100%; width: 0; background: linear-gradient(90deg, #6366f1, #8b5cf6); transition: width 0.4s ease; }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
  .panel { font-size: 13px; color: #9fb0c8; min-height: 1.4em; }
</style>
</head>
<body>
  <div class="stage">
    <div class="stepper" id="stepper" role="group" aria-label="分步进度"></div>
    <p class="panel" id="panel"></p>
    <p class="hint">点击圆点可回跳，连接线随进度填充</p>
  </div>
  <script>
    var STEPS = ["填写信息", "选择模板", "确认订单", "完成"];
    var NOTES = ["正在填写基础信息…", "挑选一套喜欢的模板。", "核对并确认订单明细。", "全部搞定，开始使用！"];
    var stepper = document.getElementById("stepper");
    var panel = document.getElementById("panel");
    var nodes = [];
    var tracks = [];

    STEPS.forEach(function (label, index) {
      var node = document.createElement("div");
      node.className = "node";
      node.innerHTML = '<button class="dot" aria-label="' + label + '">' + (index + 1) + '</button><span class="label">' + label + "</span>";
      node.querySelector(".dot").addEventListener("click", function () { go(index); });
      stepper.appendChild(node);
      nodes.push(node);
      if (index < STEPS.length - 1) {
        var track = document.createElement("div");
        track.className = "track";
        track.innerHTML = "<i></i>";
        stepper.appendChild(track);
        tracks.push(track);
      }
    });

    var current = 1;
    function go(index) {
      current = index;
      nodes.forEach(function (node, i) {
        node.classList.toggle("done", i < current);
        node.classList.toggle("current", i === current);
        node.querySelector(".dot").textContent = i < current ? "✓" : i + 1;
      });
      tracks.forEach(function (track, i) {
        track.querySelector("i").style.width = i < current ? "100%" : "0";
      });
      panel.textContent = NOTES[current];
    }
    go(current);
  </script>
</body>
</html>`;

export const stepperAsset: AssetManifest = {
  id: "reactbits-stepper",
  title: "分步进度",
  titleEn: "Stepper",
  description: "步骤圆点 + 连接线填充推进，完成态打勾，可点击回跳已走过的步骤。",
  descriptionEn: "Dot-and-track stepper with fill animation and jump-back.",
  category: "control",
  tags: ["react-bits", "控件", "步骤条", "进度", "引导"],
  previewHtml: HTML,
  files: [{ name: "reactbits-stepper.html", language: "html", content: HTML }],
  prompt:
    "请把「分步进度」装进我的项目：横向步骤条——每步一个圆点（当前步外圈辉光、已完步实底打勾）+ 步骤名，步与步之间一条 3px 轨道线随进度用渐变填充（width 过渡 0.4s）；允许点击圆点回跳任意已到达步骤；aria-label 标注每步名称与当前状态；键盘可达（button 语义）。先看现有表单/引导流程，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/components/stepper",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
