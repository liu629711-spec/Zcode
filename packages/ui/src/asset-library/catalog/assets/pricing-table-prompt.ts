import type { AssetManifest } from "../types.js";
import { renderPromptCard } from "../prompt-card.js";

/**
 * 「三档定价表」口令卡（自制原创）：prompt 类货——口令本身就是货。
 * 说明页用共享模板（V2-3 重做：徽标 + 标题 + 说明 + 口令全文排版页）。
 */

const PROMPT =
  "请为我的项目做一张「三档定价表」：免费/专业/团队三张定价卡并排，中间「专业」档抬升强调——渐变描边加顶部「最受欢迎」胶囊角标；每卡含档位名、价格大数字、账期说明、4~6 行功能清单（✓ 与 — 两种状态）；顶部一个「月付/年付」分段切换，切年付时价格数字平滑滚动到折扣价并显示「省 20%」徽标。要求：移动端纵向堆叠且强调档排最前、切换态用 aria-pressed、动效尊重 prefers-reduced-motion；先看现有的卡片与开关组件尽量复用，融入而不是覆盖。";

export const pricingTableAsset: AssetManifest = {
  id: "pricing-table-prompt",
  title: "三档定价表",
  titleEn: "Pricing Table",
  description: "口令卡：三档定价卡加月/年付切换的成套口令。",
  descriptionEn: "A prompt card for a three-tier pricing table with billing toggle.",
  category: "prompt",
  tags: ["prompt", "定价表", "营销页", "口令"],
  previewHtml: renderPromptCard(
    { title: "三档定价表", prompt: PROMPT },
    "不附带代码图纸，<b>口令本身就是货</b>。发进会话后，智能体会现做三档定价卡：中间档「最受欢迎」强调、月付/年付切换带折扣滚动价、功能清单勾叉分明。",
  ),
  files: [],
  prompt: PROMPT,
};
