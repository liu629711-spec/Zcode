import type { AssetManifest } from "./types.js";
import { assembleOpenSourcePreview } from "./previewAssemble.js";
import { BeautifulUiLoadingStateAsset } from "./assets/beautifului-loading-state.js";
import { BeautifulUiThinkingAsset } from "./assets/beautifului-thinking.js";
import { BeautifulUiStreamingTextAsset } from "./assets/beautifului-streaming-text.js";
import { BeautifulUiApprovalCardAsset } from "./assets/beautifului-approval-card.js";
import { BeautifulUiToolChipsAsset } from "./assets/beautifului-tool-chips.js";
import { BeautifulUiTaskRowsAsset } from "./assets/beautifului-task-rows.js";
import { BeautifulUiChatAsset } from "./assets/beautifului-chat.js";
import { BeautifulUiPromptBarAsset } from "./assets/beautifului-prompt-bar.js";
import { BeautifulUiRecommendationCardAsset } from "./assets/beautifului-recommendation-card.js";
import { BeautifulUiContextCardsAsset } from "./assets/beautifului-context-cards.js";
import { BeautifulUiDiffTableAsset } from "./assets/beautifului-diff-table.js";
import { BeautifulUiRecordsTableAsset } from "./assets/beautifului-records-table.js";
import { BeautifulUiFilterTableAsset } from "./assets/beautifului-filter-table.js";
import { BeautifulUiSidebarNavAsset } from "./assets/beautifului-sidebar-nav.js";
import { BeautifulUiSearchAsset } from "./assets/beautifului-search.js";
import { BeautifulUiInsightCardsAsset } from "./assets/beautifului-insight-cards.js";
import { BeautifulUiCodeBlockAsset } from "./assets/beautifului-code-block.js";
import { BeautifulUiFineTuneCardAsset } from "./assets/beautifului-fine-tune-card.js";
import { BeautifulUiSelectionActionsAsset } from "./assets/beautifului-selection-actions.js";
import { RareUiAnimatedTabAsset } from "./assets/rareui-animated-tab.js";
import { RareUiGlassShimmerButtonAsset } from "./assets/rareui-glass-shimmer-button.js";
import { RareUiLiquidButtonAsset } from "./assets/rareui-liquid-button.js";
import { RareUiLoadingSpinnerAsset } from "./assets/rareui-loading-spinner.js";
import { RareUiLiquidTooltipAsset } from "./assets/rareui-liquid-tooltip.js";
import { RareUiPremiumButtonAsset } from "./assets/rareui-premium-button.js";
import { RareUiSoftButtonAsset } from "./assets/rareui-soft-button.js";
import { RareUiNeumorphism3DButtonAsset } from "./assets/rareui-neumorphism-3d-button.js";
import { RareUiRetroPixelButtonAsset } from "./assets/rareui-retro-pixel-button.js";
import { RareUiFeatureBadgeAsset } from "./assets/rareui-feature-badge.js";
import { RareUiFloatingNavigationAsset } from "./assets/rareui-floating-navigation.js";
import { RareUiToastTabsAsset } from "./assets/rareui-toast-tabs.js";
import { RareUiParticleCardAsset } from "./assets/rareui-particle-card.js";
import { RareUiPremiumProfileCardAsset } from "./assets/rareui-premium-profile-card.js";
import { RareUiSoundTextAsset } from "./assets/rareui-sound-text.js";
import { RareUiWordMagnetAsset } from "./assets/rareui-word-magnet.js";
import { RareUiMagneticScatterTextAsset } from "./assets/rareui-magnetic-scatter-text.js";
import { RareUiVaporSmokeTextAsset } from "./assets/rareui-vapor-smoke-text.js";
import { RareUiImageExpandTestimonialAsset } from "./assets/rareui-image-expand-testimonial.js";
import { RareUiBook3DAsset } from "./assets/rareui-book-3d.js";
import { RareUiLiquidMetalAsset } from "./assets/rareui-liquid-metal.js";
import { RareUiLiquidWaveAsset } from "./assets/rareui-liquid-wave.js";
import { RareUiThreeDButtonAsset } from "./assets/rareui-three-d-button.js";
import { RareUiProfileDropdownAsset } from "./assets/rareui-profile-dropdown.js";
import { RareUiGlassSearchBarAsset } from "./assets/rareui-glass-search-bar.js";
import { RareUiAvatarGroupAsset } from "./assets/rareui-avatar-group.js";
import { UIverseCtaArrowButtonAsset } from "./assets/uiverse-cta-arrow-button.js";
import { UIverseSparkleGenerateButtonAsset } from "./assets/uiverse-sparkle-generate-button.js";
import { UIverseLiquidFillButtonAsset } from "./assets/uiverse-liquid-fill-button.js";
import { UIverseRainbowGlowButtonAsset } from "./assets/uiverse-rainbow-glow-button.js";
import { UIverseBlockMosaicLoaderAsset } from "./assets/uiverse-block-mosaic-loader.js";
import { UIverseBarSweepLoaderAsset } from "./assets/uiverse-bar-sweep-loader.js";
import { UIverseDualRingLoaderAsset } from "./assets/uiverse-dual-ring-loader.js";
import { UIverseDayNightLetterToggleAsset } from "./assets/uiverse-day-night-letter-toggle.js";
import { UIverseIosStyleSwitchAsset } from "./assets/uiverse-ios-style-switch.js";
import { UIverseGooeyColorSwitchAsset } from "./assets/uiverse-gooey-color-switch.js";
import { UIverseThumbCheckboxAsset } from "./assets/uiverse-thumb-checkbox.js";
import { UIverseClassicCheckboxAsset } from "./assets/uiverse-classic-checkbox.js";
import { UIverseMaterialTooltipAsset } from "./assets/uiverse-material-tooltip.js";
import { UIverseSwapTextNotificationAsset } from "./assets/uiverse-swap-text-notification.js";
import { UIverseSlideRevealCardAsset } from "./assets/uiverse-slide-reveal-card.js";

/**
 * V3-3 逐字收录货的注册表（Beautiful UI 19 / RareUI 26 / UIverse 15，共 60 件；
 * 技术设计 §10 V2-4/V3-3）。
 *
 * 单独成文件的原因：60 条 import + 60 条条目会把 catalog/index.ts 顶过 oxlint 的
 * max-lines(520) 上限，而 index.ts 同时还要挂 143 张设计风格卡。
 *
 * previewHtml 由 previewAssemble 按需拼装（构建期产物 PREVIEW_RUNTIME_CHUNKS +
 * OPEN_SOURCE_PREVIEWS），模块级缓存——同一件只拼一次字符串，60 件不必各自内联
 * 一份 react（那会让 preview-html.ts 涨到十几 MB）。图纸（files）是上游源码逐字，
 * 本表不动它，只补运行时预览；UIverse 货是纯 HTML/CSS 片段（无 preview 字段），
 * previewHtml 直接写在卡里。
 */
export const OPEN_SOURCE_ASSETS: AssetManifest[] = [
  BeautifulUiLoadingStateAsset,
  BeautifulUiThinkingAsset,
  BeautifulUiStreamingTextAsset,
  BeautifulUiApprovalCardAsset,
  BeautifulUiToolChipsAsset,
  BeautifulUiTaskRowsAsset,
  BeautifulUiChatAsset,
  BeautifulUiPromptBarAsset,
  BeautifulUiRecommendationCardAsset,
  BeautifulUiContextCardsAsset,
  BeautifulUiDiffTableAsset,
  BeautifulUiRecordsTableAsset,
  BeautifulUiFilterTableAsset,
  BeautifulUiSidebarNavAsset,
  BeautifulUiSearchAsset,
  BeautifulUiInsightCardsAsset,
  BeautifulUiCodeBlockAsset,
  BeautifulUiFineTuneCardAsset,
  BeautifulUiSelectionActionsAsset,
  RareUiAnimatedTabAsset,
  RareUiGlassShimmerButtonAsset,
  RareUiLiquidButtonAsset,
  RareUiLoadingSpinnerAsset,
  RareUiLiquidTooltipAsset,
  RareUiPremiumButtonAsset,
  RareUiSoftButtonAsset,
  RareUiNeumorphism3DButtonAsset,
  RareUiRetroPixelButtonAsset,
  RareUiFeatureBadgeAsset,
  RareUiFloatingNavigationAsset,
  RareUiToastTabsAsset,
  RareUiParticleCardAsset,
  RareUiPremiumProfileCardAsset,
  RareUiSoundTextAsset,
  RareUiWordMagnetAsset,
  RareUiMagneticScatterTextAsset,
  RareUiVaporSmokeTextAsset,
  RareUiImageExpandTestimonialAsset,
  RareUiBook3DAsset,
  RareUiLiquidMetalAsset,
  RareUiLiquidWaveAsset,
  RareUiThreeDButtonAsset,
  RareUiProfileDropdownAsset,
  RareUiGlassSearchBarAsset,
  RareUiAvatarGroupAsset,
  UIverseCtaArrowButtonAsset,
  UIverseSparkleGenerateButtonAsset,
  UIverseLiquidFillButtonAsset,
  UIverseRainbowGlowButtonAsset,
  UIverseBlockMosaicLoaderAsset,
  UIverseBarSweepLoaderAsset,
  UIverseDualRingLoaderAsset,
  UIverseDayNightLetterToggleAsset,
  UIverseIosStyleSwitchAsset,
  UIverseGooeyColorSwitchAsset,
  UIverseThumbCheckboxAsset,
  UIverseClassicCheckboxAsset,
  UIverseMaterialTooltipAsset,
  UIverseSwapTextNotificationAsset,
  UIverseSlideRevealCardAsset,
].map((asset) =>
  asset.preview ? { ...asset, previewHtml: assembleOpenSourcePreview(asset.id) } : asset,
);
