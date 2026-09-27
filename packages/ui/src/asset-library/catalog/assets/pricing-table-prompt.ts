import type { AssetManifest } from "../types.js";

/**
 * 「三档定价表」口令卡（自制原创）：prompt 类货——口令本身就是货。
 */

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>三档定价表 · 口令卡</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: linear-gradient(150deg, #1a1208 0%, #2b2113 55%, #140f08 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #e8dfcf;
  }
  .note {
    width: min(420px, 88vw); padding: 28px 30px; border-radius: 20px;
    background: rgb(255 255 255 / 0.06); border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(14px);
  }
  h1 { margin: 0 0 6px; font-size: 20px; }
  .kind { font-size: 12px; color: #fbbf24; letter-spacing: 0.12em; }
  p { margin: 14px 0 0; font-size: 14px; line-height: 1.8; color: #c4b8a0; }
  b { color: #f5eddc; }
</style>
</head>
<body>
  <div class="note">
    <div class="kind">口令卡 · PROMPT</div>
    <h1>三档定价表</h1>
    <p>这是一件<b>口令货</b>：不附带代码图纸，<b>口令本身就是货</b>。</p>
    <p>复制或发进会话后，智能体会按口令现做三档定价卡：中间档「最受欢迎」强调、月付/年付切换带折扣滚动价、功能清单勾叉分明。</p>
    <p>适合放在<b>新会话</b>里直接点单，档位与价格换成你自己的套餐即可。</p>
  </div>
</body>
</html>`;

export const pricingTableAsset: AssetManifest = {
  id: "pricing-table-prompt",
  title: "三档定价表",
  titleEn: "Pricing Table",
  description: "口令卡：三档定价卡加月/年付切换的成套口令。",
  descriptionEn: "A prompt card for a three-tier pricing table with billing toggle.",
  category: "prompt",
  tags: ["prompt", "定价表", "营销页", "口令"],
  previewHtml: PREVIEW_HTML,
  files: [],
  prompt:
    "请为我的项目做一张「三档定价表」：免费/专业/团队三张定价卡并排，中间「专业」档抬升强调——渐变描边加顶部「最受欢迎」胶囊角标；每卡含档位名、价格大数字、账期说明、4~6 行功能清单（✓ 与 — 两种状态）；顶部一个「月付/年付」分段切换，切年付时价格数字平滑滚动到折扣价并显示「省 20%」徽标。要求：移动端纵向堆叠且强调档排最前、切换态用 aria-pressed、动效尊重 prefers-reduced-motion；先看现有的卡片与开关组件尽量复用，融入而不是覆盖。",
};
