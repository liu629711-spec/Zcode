import type { AssetManifest } from "./types.js";
import { breathingButtonAsset } from "./assets/breathing-button.js";
import { frostedGlassLoginAsset } from "./assets/frosted-glass-login-prompt.js";
import { jellyToggleAsset } from "./assets/jelly-toggle.js";
import { meteorBackgroundAsset } from "./assets/meteor-background.js";
import { skeletonShimmerAsset } from "./assets/skeleton-shimmer.js";
import { typewriterAsset } from "./assets/typewriter-text.js";

/**
 * 内置目录（技术设计 §1）。一期全是 html/prompt 类货，previewHtml 直接写在
 * 资产文件里；react 货走 preview-html.ts 生成产物的机制归 S2，到时在这里合并。
 * 二期"会话产物入库"的唯一合并点：`[...ASSET_CATALOG, ...userAssets]`（一期不实现）。
 */
export const ASSET_CATALOG: AssetManifest[] = [
  jellyToggleAsset,
  typewriterAsset,
  meteorBackgroundAsset,
  breathingButtonAsset,
  skeletonShimmerAsset,
  frostedGlassLoginAsset,
];
