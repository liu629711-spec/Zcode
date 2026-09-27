import type { AssetManifest } from "./types.js";
import { breathingButtonAsset } from "./assets/breathing-button.js";
import { frostedGlassLoginAsset } from "./assets/frosted-glass-login-prompt.js";
import { glowBorderCardAsset } from "./assets/glow-border-card.js";
import { jellyToggleAsset } from "./assets/jelly-toggle.js";
import { meteorBackgroundAsset } from "./assets/meteor-background.js";
import { numberRollAsset } from "./assets/number-roll.js";
import { skeletonShimmerAsset } from "./assets/skeleton-shimmer.js";
import { typewriterAsset } from "./assets/typewriter-text.js";
import { REACT_PREVIEW_HTML } from "./preview-html.js";

/**
 * react 货的 previewHtml 从构建期产物接线（scripts/build-asset-previews.mjs 生成）。
 * 产物缺失时置空串——validateCatalog 会报"必填字段 previewHtml 为空"让测试炸掉，
 * 而不是静默白屏（重新生成：仓库根 node scripts/build-asset-previews.mjs）。
 */
const reactAssets: AssetManifest[] = [glowBorderCardAsset, numberRollAsset].map((asset) => ({
  ...asset,
  previewHtml: REACT_PREVIEW_HTML[asset.id] ?? "",
}));

/**
 * 内置目录（技术设计 §1）。html/prompt 类货 previewHtml 直接写在资产文件里，
 * react 类货走上面的接线。二期"会话产物入库"的唯一合并点：
 * `[...ASSET_CATALOG, ...userAssets]`（一期不实现）。
 */
export const ASSET_CATALOG: AssetManifest[] = [
  jellyToggleAsset,
  typewriterAsset,
  meteorBackgroundAsset,
  breathingButtonAsset,
  skeletonShimmerAsset,
  frostedGlassLoginAsset,
  ...reactAssets,
];
