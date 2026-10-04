import type { AssetManifest } from "./types.js";
import { DESIGN_STYLE_ASSETS } from "./designStyleAssets.js";
import { aiApprovalCardAsset } from "./assets/ai-approval-card.js";
import { aiLoadingDotsAsset } from "./assets/ai-loading-dots.js";
import { aiStreamedTextAsset } from "./assets/ai-streamed-text.js";
import { aiTaskRowAsset } from "./assets/ai-task-row.js";
import { aiThinkingTraceAsset } from "./assets/ai-thinking-trace.js";
import { aiToolCallRowAsset } from "./assets/ai-tool-call-row.js";
import { animatedListAsset } from "./assets/animated-list.js";
import { animatedTooltipAsset } from "./assets/animated-tooltip.js";
import { auroraBackgroundAsset } from "./assets/aurora-background.js";
import { blurTextAsset } from "./assets/blur-text.js";
import { breathingButtonAsset } from "./assets/breathing-button.js";
import { checkmarkCheckboxAsset } from "./assets/checkmark-checkbox.js";
import { clickSparkAsset } from "./assets/click-spark.js";
import { commandPaletteAsset } from "./assets/command-palette-prompt.js";
import { countUpAsset } from "./assets/count-up.js";
import { dashboardKpiAsset } from "./assets/dashboard-kpi-prompt.js";
import { dayNightToggleAsset } from "./assets/day-night-toggle.js";
import { emptyStateAsset } from "./assets/empty-state-prompt.js";
import { flipCardAsset } from "./assets/flip-card.js";
import { flipClockAsset } from "./assets/flip-clock.js";
import { frostedGlassLoginAsset } from "./assets/frosted-glass-login-prompt.js";
import { glassPanelAsset } from "./assets/glass-panel.js";
import { glitchTextAsset } from "./assets/glitch-text.js";
import { glowBorderCardAsset } from "./assets/glow-border-card.js";
import { glowFocusInputAsset } from "./assets/glow-focus-input.js";
import { gradientFlowTextAsset } from "./assets/gradient-flow-text.js";
import { hamburgerMorphAsset } from "./assets/hamburger-morph.js";
import { jellyToggleAsset } from "./assets/jelly-toggle.js";
import { landingHeroAsset } from "./assets/landing-hero-prompt.js";
import { likeButtonAsset } from "./assets/like-button.js";
import { magneticButtonAsset } from "./assets/magnetic-button.js";
import { meteorBackgroundAsset } from "./assets/meteor-background.js";
import { neonButtonAsset } from "./assets/neon-button.js";
import { numberRollAsset } from "./assets/number-roll.js";
import { orbitLoaderAsset } from "./assets/orbit-loader.js";
import { particleNetworkAsset } from "./assets/particle-network.js";
import { pricingTableAsset } from "./assets/pricing-table-prompt.js";
import { radarScanAsset } from "./assets/radar-scan.js";
import { ribbonsBackgroundAsset } from "./assets/ribbons-background.js";
import { rippleButtonAsset } from "./assets/ripple-button.js";
import { segmentedControlAsset } from "./assets/segmented-control.js";
import { silkBackgroundAsset } from "./assets/silk-background.js";
import { skeletonShimmerAsset } from "./assets/skeleton-shimmer.js";
import { splitTextAsset } from "./assets/split-text.js";
import { spotlightCardAsset } from "./assets/spotlight-card.js";
import { stepProgressAsset } from "./assets/step-progress.js";
import { textScrambleAsset } from "./assets/text-scramble.js";
import { toastNotificationAsset } from "./assets/toast-notification.js";
import { typewriterAsset } from "./assets/typewriter-text.js";
import { waveBackgroundAsset } from "./assets/wave-background.js";
import { OPEN_SOURCE_ASSETS } from "./openSourceAssets.js";
import { REACT_BITS_ASSETS } from "./reactBitsAssets.js";

/**
 * react 货的 previewHtml 真身在 preview-html.ts（构建期产物，
 * scripts/build-asset-previews.mjs 生成）。瘦身拆分（2026-10-05）后这份
 * 3.96MB 的 map 不再静态吸进本模块（mentions/composer 也 import 本模块）——
 * 三件货只带 bodyFrom 标记，预览 HTML 经 assetBodies.ts 的 loadAssetBody
 * 按需 dynamic import（产物缺失时报错而不是静默白屏；
 * 重新生成：仓库根 node scripts/build-asset-previews.mjs）。
 */

const reactAssets: AssetManifest[] = [flipClockAsset, glowBorderCardAsset, numberRollAsset].map(
  (asset) => ({
    ...asset,
    previewHtml: "",
    bodyFrom: "preview-html" as const,
  }),
);

/**
 * 内置目录（技术设计 §1）。html/prompt 类货 previewHtml 直接写在资产文件里，
 * react 类货走上面的接线。二期"会话产物入库"的唯一合并点：
 * `[...ASSET_CATALOG, ...userAssets]`（一期不实现）。
 *
 * 两个不进本表的货：
 * - 设计风格 152 张在 DESIGN_STYLE_ASSETS（designStyleAssets.ts，index 行数上限）；
 * - UIverse 自动收录 3,793 件在 catalog/generated/（BULK_AUTO_ASSETS），只被展厅
 *   动态 import——体积 ~12MB，进主包会拖垮首屏，mentions/composer 也不需要它。
 */
export const ASSET_CATALOG: AssetManifest[] = [
  // S1 种子货（8 件）
  jellyToggleAsset,
  typewriterAsset,
  meteorBackgroundAsset,
  breathingButtonAsset,
  skeletonShimmerAsset,
  frostedGlassLoginAsset,
  ...reactAssets,
  // —— S5 备货批次（22 件，自制原创，按类别分组）——
  // 文字动效
  gradientFlowTextAsset,
  glitchTextAsset,
  textScrambleAsset,
  // 背景
  auroraBackgroundAsset,
  particleNetworkAsset,
  radarScanAsset,
  waveBackgroundAsset,
  // 控件
  neonButtonAsset,
  rippleButtonAsset,
  magneticButtonAsset,
  segmentedControlAsset,
  likeButtonAsset,
  hamburgerMorphAsset,
  // 区块
  glassPanelAsset,
  stepProgressAsset,
  toastNotificationAsset,
  // 提示词
  landingHeroAsset,
  commandPaletteAsset,
  dashboardKpiAsset,
  emptyStateAsset,
  pricingTableAsset,
  // —— V2-4 四站首批（20 件，效果自实现 + source 逐件标注，技术设计 §10）——
  // React Bits（MIT + Commons Clause，仅灵感参考）
  splitTextAsset,
  blurTextAsset,
  countUpAsset,
  silkBackgroundAsset,
  clickSparkAsset,
  ribbonsBackgroundAsset,
  // Beautiful UI（MIT，AI 界面零件）
  aiLoadingDotsAsset,
  aiThinkingTraceAsset,
  aiToolCallRowAsset,
  aiApprovalCardAsset,
  aiTaskRowAsset,
  aiStreamedTextAsset,
  // RareUI（MIT，shadcn 风动效）
  animatedTooltipAsset,
  spotlightCardAsset,
  flipCardAsset,
  animatedListAsset,
  // UIverse（CC BY 4.0，小控件）
  dayNightToggleAsset,
  orbitLoaderAsset,
  checkmarkCheckboxAsset,
  glowFocusInputAsset,
  // —— V3-3 三站逐字收录（Beautiful UI 19 / RareUI 26 / UIverse 15，共 60 件）——
  ...OPEN_SOURCE_ASSETS,
  // —— V5 React Bits 效果自实现扩批（Commons Clause 禁逐字，灵感参考口径不变）——
  ...REACT_BITS_ASSETS,
  // —— V3-3 设计风格库（open-design，152 张，逐字收录 DESIGN.md）——
  ...DESIGN_STYLE_ASSETS,
];
