import type { AssetManifest } from "../types.js";

/**
 * 「仪表盘 KPI 卡组」口令卡（自制原创）：prompt 类货——口令本身就是货。
 */

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>仪表盘 KPI 卡组 · 口令卡</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: linear-gradient(150deg, #0a1418 0%, #10222b 55%, #08111a 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #dbe4f0;
  }
  .note {
    width: min(420px, 88vw); padding: 28px 30px; border-radius: 20px;
    background: rgb(255 255 255 / 0.06); border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(14px);
  }
  h1 { margin: 0 0 6px; font-size: 20px; }
  .kind { font-size: 12px; color: #5eead4; letter-spacing: 0.12em; }
  p { margin: 14px 0 0; font-size: 14px; line-height: 1.8; color: #aebad0; }
  b { color: #e6edf5; }
</style>
</head>
<body>
  <div class="note">
    <div class="kind">口令卡 · PROMPT</div>
    <h1>仪表盘 KPI 卡组</h1>
    <p>这是一件<b>口令货</b>：不附带代码图纸，<b>口令本身就是货</b>。</p>
    <p>复制或发进会话后，智能体会按口令现做一排 KPI 指标卡：大数字、环比徽标、迷你趋势 sparkline、加载骨架与空值占位全部在口令里约定。</p>
    <p>适合放在<b>新会话</b>里直接点单，指标名与数据源换成你自己的即可。</p>
  </div>
</body>
</html>`;

export const dashboardKpiAsset: AssetManifest = {
  id: "dashboard-kpi-prompt",
  title: "仪表盘 KPI 卡组",
  titleEn: "Dashboard KPI Cards",
  description: "口令卡：带 sparkline 与环比徽标的 KPI 卡片组。",
  descriptionEn: "A prompt card for KPI cards with sparklines and deltas.",
  category: "prompt",
  tags: ["prompt", "仪表盘", "数据", "口令"],
  previewHtml: PREVIEW_HTML,
  files: [],
  prompt:
    "请为我的项目做一组「仪表盘 KPI 卡片」：一排四张等宽指标卡，每张含——指标名（小字灰）、主数值（大号等宽数字）、环比徽标（涨绿跌红带 ↑↓ 箭头）、右下一枚 60×24 的迷你趋势 sparkline（SVG 折线，涨绿跌红）；卡片悬停轻微上浮加描边提亮；数据全部从 props/接口接入，并提供加载骨架与空值占位「—」。要求：数值变化时做一次 300ms 淡入过渡，尊重 prefers-reduced-motion；先看现有的数据展示与图表库约定，融入而不是覆盖。",
};
