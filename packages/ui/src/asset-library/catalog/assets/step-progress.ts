import type { AssetManifest } from "../types.js";

/**
 * 步骤条（自制原创）：圆节点 + 连接轨道，JS 驱动 上一步/下一步，
 * 已完成填充渐变、当前高亮、轨道填充平滑过渡。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>步骤条</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(110% 110% at 50% 0%, #101827 0%, #0a0f18 65%, #070b12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #cdd8ea;
  }
  .stepper { width: min(460px, 90vw); }
  .rail { position: relative; height: 40px; }
  .track, .fill {
    position: absolute; top: 19px; height: 3px; border-radius: 3px;
  }
  .track { left: 20px; right: 20px; background: #1c2536; }
  .fill {
    left: 20px; width: 0;
    background: linear-gradient(90deg, #34d399, #38bdf8);
    transition: width 0.4s ease;
  }
  .steps { position: relative; display: flex; justify-content: space-between; margin-top: -40px; padding: 0; list-style: none; }
  .steps li { display: flex; flex-direction: column; align-items: center; gap: 8px; width: 76px; }
  .steps i {
    width: 34px; height: 34px; border-radius: 50%; display: grid; place-items: center;
    font-style: normal; font-size: 14px; background: #141b28; border: 2px solid #2a3550;
    transition: border-color 0.3s, background 0.3s, box-shadow 0.3s;
  }
  .steps span { font-size: 12px; color: #7c8db0; }
  .steps .done i { background: linear-gradient(135deg, #34d399, #38bdf8); border-color: transparent; color: #062018; font-weight: 700; }
  .steps .done span { color: #9fb0c9; }
  .steps .current i { border-color: #38bdf8; box-shadow: 0 0 0 4px rgb(56 189 248 / 0.18); color: #7dd3fc; font-weight: 700; }
  .steps .current span { color: #cdd8ea; }
  .ops { display: flex; gap: 10px; justify-content: center; margin-top: 26px; }
  .ops button {
    padding: 8px 20px; border-radius: 9px; cursor: pointer; font-size: 13px;
    border: 1px solid #2a3550; background: #141b28; color: #cdd8ea;
  }
  .ops button:disabled { opacity: 0.4; cursor: not-allowed; }
  .ops button:not(:disabled):focus-visible { outline: 2px solid #60a5fa; outline-offset: 2px; }
  @media (prefers-reduced-motion: reduce) { .fill, .steps i { transition: none; } }
</style>
</head>
<body>
  <div class="stepper">
    <div class="rail"><div class="track"></div><div class="fill" id="fill"></div></div>
    <ol class="steps" id="steps">
      <li class="done"><i>1</i><span>填写信息</span></li>
      <li class="current" aria-current="step"><i>2</i><span>确认订单</span></li>
      <li><i>3</i><span>支付</span></li>
      <li><i>4</i><span>完成</span></li>
    </ol>
    <div class="ops">
      <button type="button" id="prev">上一步</button>
      <button type="button" id="next">下一步</button>
    </div>
  </div>
  <script>
    var items = document.querySelectorAll("#steps > li");
    var fill = document.getElementById("fill");
    var prev = document.getElementById("prev");
    var next = document.getElementById("next");
    var current = 1; // 从 0 数
    function render() {
      for (var i = 0; i < items.length; i++) {
        var li = items[i];
        li.className = i < current ? "done" : i === current ? "current" : "";
        if (i === current) li.setAttribute("aria-current", "step");
        else li.removeAttribute("aria-current");
        li.querySelector("i").textContent = i < current ? "\\u2713" : String(i + 1);
      }
      fill.style.width = (current / (items.length - 1)) * 100 + "%";
      prev.disabled = current === 0;
      next.disabled = current === items.length - 1;
    }
    prev.addEventListener("click", function () { current = Math.max(0, current - 1); render(); });
    next.addEventListener("click", function () { current = Math.min(items.length - 1, current + 1); render(); });
    render();
  </script>
</body>
</html>`;

export const stepProgressAsset: AssetManifest = {
  id: "step-progress",
  title: "步骤条",
  titleEn: "Step Progress",
  description: "可上一步下一步的分步进度条，节点随状态点亮。",
  descriptionEn: "An interactive stepper whose nodes light up with progress.",
  category: "block",
  tags: ["js", "步骤条", "进度", "表单"],
  previewHtml: HTML,
  files: [{ name: "step-progress.html", language: "html", content: HTML }],
  prompt:
    "请把「步骤条」装进我的项目：一个可交互的分步进度组件——横向 N 步，圆节点加连接轨道，已完成节点填充品牌渐变并打勾、当前节点高亮描边带光圈、未来节点灰态，轨道填充长度随进度平滑过渡；提供上一步/下一步按钮驱动状态（业务接入时替换为真实流转）；节点带 aria-current=\"step\"；尊重 prefers-reduced-motion（进度瞬切）。先看现有的表单/向导流程，融入而不是覆盖。",
};
