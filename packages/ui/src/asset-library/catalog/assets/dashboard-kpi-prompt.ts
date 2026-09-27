import type { AssetManifest } from "../types.js";
import { renderPromptCard } from "../prompt-card.js";

/**
 * 「仪表盘 KPI 卡组」口令卡（自制原创）：prompt 类货——口令本身就是货。
 * 说明页用共享模板（V2-3 重做：徽标 + 标题 + 说明 + 口令全文排版页）。
 */

const PROMPT =
  "请为我的项目做一组「仪表盘 KPI 卡片」：一排四张等宽指标卡，每张含——指标名（小字灰）、主数值（大号等宽数字）、环比徽标（涨绿跌红带 ↑↓ 箭头）、右下一枚 60×24 的迷你趋势 sparkline（SVG 折线，涨绿跌红）；卡片悬停轻微上浮加描边提亮；数据全部从 props/接口接入，并提供加载骨架与空值占位「—」。要求：数值变化时做一次 300ms 淡入过渡，尊重 prefers-reduced-motion；先看现有的数据展示与图表库约定，融入而不是覆盖。";

export const dashboardKpiAsset: AssetManifest = {
  id: "dashboard-kpi-prompt",
  title: "仪表盘 KPI 卡组",
  titleEn: "Dashboard KPI Cards",
  description: "口令卡：带 sparkline 与环比徽标的 KPI 卡片组。",
  descriptionEn: "A prompt card for KPI cards with sparklines and deltas.",
  category: "prompt",
  tags: ["prompt", "仪表盘", "数据", "口令"],
  previewHtml: renderPromptCard(
    { title: "仪表盘 KPI 卡组", prompt: PROMPT },
    "不附带代码图纸，<b>口令本身就是货</b>。发进会话后，智能体会现做一排 KPI 指标卡：大数字、环比徽标、迷你趋势 sparkline、加载骨架与空值占位全部在口令里约定。",
  ),
  files: [],
  prompt: PROMPT,
};
