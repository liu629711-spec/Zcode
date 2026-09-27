import type { AssetManifest } from "../types.js";

/**
 * 环轨加载器（V2-4 首批收录，灵感来自 UIverse 的 Orbit Loader）：
 * 双卫星以不同半径/速度绕核旋转，核心呼吸发光，纯 CSS。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：role=status 加载语义；demo 提供触发小按钮演示状态切换。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>环轨加载器</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: flex; flex-direction: column; align-items: center; gap: 30px; }
  .orbit { position: relative; width: 96px; height: 96px; }
  .core {
    position: absolute; left: 50%; top: 50%; width: 16px; height: 16px; margin: -8px;
    border-radius: 50%; background: #7dd3fc;
    box-shadow: 0 0 16px rgb(125 211 252 / 0.8);
    animation: core 1.6s ease-in-out infinite;
  }
  @keyframes core { 0%, 100% { transform: scale(1); opacity: 0.85; } 50% { transform: scale(1.35); opacity: 1; } }
  .ring { position: absolute; left: 50%; top: 50%; border: 1px dashed rgb(148 163 184 / 0.25); border-radius: 50%; }
  .r1 { width: 56px; height: 56px; margin: -28px; animation: spin 2.4s linear infinite; }
  .r2 { width: 88px; height: 88px; margin: -44px; animation: spin 4.2s linear infinite reverse; }
  .sat { position: absolute; left: 50%; top: 0; border-radius: 50%; }
  .r1 .sat { width: 9px; height: 9px; margin: -4.5px; background: #a5b4fc; box-shadow: 0 0 9px rgb(165 180 252 / 0.9); }
  .r2 .sat { width: 6px; height: 6px; margin: -3px; background: #67e8f9; box-shadow: 0 0 8px rgb(103 232 249 / 0.9); }
  @keyframes spin { to { transform: rotate(360deg); } }
  .toggle {
    padding: 8px 20px; border-radius: 9px; cursor: pointer; font-size: 12.5px;
    border: 1px solid #2a3550; background: #141b28; color: #cdd8ea;
  }
  .toggle:focus-visible { outline: 2px solid #60a5fa; outline-offset: 2px; }
  .off .orbit { opacity: 0.25; filter: grayscale(0.6); }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .r1, .r2, .core { animation: none; } }
</style>
</head>
<body>
  <div class="stage" id="stage">
    <div class="orbit" role="status" aria-label="加载中">
      <span class="ring r1"><span class="sat"></span></span>
      <span class="ring r2"><span class="sat"></span></span>
      <span class="core"></span>
    </div>
    <button class="toggle" type="button" id="toggle">切换到空闲态</button>
  </div>
  <p class="hint">按钮切换加载 / 空闲两种状态</p>
  <script>
    document.getElementById("toggle").addEventListener("click", function () {
      var stage = document.getElementById("stage");
      var off = stage.classList.toggle("off");
      this.textContent = off ? "切换到加载态" : "切换到空闲态";
      document.querySelector(".orbit").setAttribute("aria-label", off ? "空闲" : "加载中");
    });
  </script>
</body>
</html>`;

export const orbitLoaderAsset: AssetManifest = {
  id: "orbit-loader",
  title: "环轨加载器",
  titleEn: "Orbit Loader",
  description: "双卫星绕核旋转的加载指示，虚线轨道可辨速度差。",
  descriptionEn: "Two satellites orbiting a pulsing core.",
  category: "control",
  tags: ["css", "loader", "加载态", "uiverse", "科幻"],
  previewHtml: HTML,
  files: [{ name: "orbit-loader.html", language: "html", content: HTML }],
  prompt:
    "请把「环轨加载器」装进我的项目：一个加载指示组件——中心发光核呼吸（scale+透明度），两条虚线轨道环各带一颗发光卫星反向旋转（内圈 2.4s、外圈 4.2s 速度差）；容器 role=\"status\" 带无障碍标签；提供加载/空闲两态切换（空闲降透明降饱和）；尊重 prefers-reduced-motion（全部静止）。适合等待智能体响应的场景。先看现有的 loading 体系，融入而不是覆盖。",
  source: {
    site: "UIverse",
    url: "https://uiverse.io",
    license: "CC BY 4.0（按件署名，效果自实现）",
  },
};
