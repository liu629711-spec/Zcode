import type { AssetManifest } from "./types.js";
import { auroraBackgroundAsset } from "./assets/aurora-background.js";
import { breathingButtonAsset } from "./assets/breathing-button.js";
import { commandPaletteAsset } from "./assets/command-palette-prompt.js";
import { dashboardKpiAsset } from "./assets/dashboard-kpi-prompt.js";
import { emptyStateAsset } from "./assets/empty-state-prompt.js";
import { flipClockAsset } from "./assets/flip-clock.js";
import { frostedGlassLoginAsset } from "./assets/frosted-glass-login-prompt.js";
import { glassPanelAsset } from "./assets/glass-panel.js";
import { glitchTextAsset } from "./assets/glitch-text.js";
import { glowBorderCardAsset } from "./assets/glow-border-card.js";
import { gradientFlowTextAsset } from "./assets/gradient-flow-text.js";
import { hamburgerMorphAsset } from "./assets/hamburger-morph.js";
import { jellyToggleAsset } from "./assets/jelly-toggle.js";
import { landingHeroAsset } from "./assets/landing-hero-prompt.js";
import { likeButtonAsset } from "./assets/like-button.js";
import { magneticButtonAsset } from "./assets/magnetic-button.js";
import { meteorBackgroundAsset } from "./assets/meteor-background.js";
import { neonButtonAsset } from "./assets/neon-button.js";
import { numberRollAsset } from "./assets/number-roll.js";
import { particleNetworkAsset } from "./assets/particle-network.js";
import { pricingTableAsset } from "./assets/pricing-table-prompt.js";
import { radarScanAsset } from "./assets/radar-scan.js";
import { rippleButtonAsset } from "./assets/ripple-button.js";
import { segmentedControlAsset } from "./assets/segmented-control.js";
import { skeletonShimmerAsset } from "./assets/skeleton-shimmer.js";
import { stepProgressAsset } from "./assets/step-progress.js";
import { textScrambleAsset } from "./assets/text-scramble.js";
import { toastNotificationAsset } from "./assets/toast-notification.js";
import { typewriterAsset } from "./assets/typewriter-text.js";
import { waveBackgroundAsset } from "./assets/wave-background.js";
import { REACT_PREVIEW_HTML } from "./preview-html.js";

/**
 * react 货的 previewHtml 从构建期产物接线（scripts/build-asset-previews.mjs 生成）。
 * 产物缺失时置空串——validateCatalog 会报"必填字段 previewHtml 为空"让测试炸掉，
 * 而不是静默白屏（重新生成：仓库根 node scripts/build-asset-previews.mjs）。
 */
const reactAssets: AssetManifest[] = [flipClockAsset, glowBorderCardAsset, numberRollAsset].map(
  (asset) => ({
    ...asset,
    previewHtml: REACT_PREVIEW_HTML[asset.id] ?? "",
  }),
);

/**
 * 内置目录（技术设计 §1）。html/prompt 类货 previewHtml 直接写在资产文件里，
 * react 类货走上面的接线。二期"会话产物入库"的唯一合并点：
 * `[...ASSET_CATALOG, ...userAssets]`（一期不实现）。
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
];
