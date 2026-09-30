/**
 * React Bits 灵感批次注册表（V5 扩批，21 件）。
 *
 * React Bits 许可为 MIT + Commons Clause v1.0——明文禁止把组件本身
 * （单独/打包/移植版）再分发，因此本批全部为效果自实现：只参考效果观感，
 * 代码逐字未复制、未移植上游实现，卡片 source 里逐件标注出处与口径。
 * 逐卡文件在 assets/reactbits-*.ts。
 */
import type { AssetManifest } from "./types.js";
import { circularTextAsset } from "./assets/reactbits-circular-text.js";
import { decryptedTextAsset } from "./assets/reactbits-decrypted-text.js";
import { dockAsset } from "./assets/reactbits-dock.js";
import { dotGridAsset } from "./assets/reactbits-dot-grid.js";
import { electricBorderAsset } from "./assets/reactbits-electric-border.js";
import { foldTextAsset } from "./assets/reactbits-fold-text.js";
import { fuzzyTextAsset } from "./assets/reactbits-fuzzy-text.js";
import { glareHoverAsset } from "./assets/reactbits-glare-hover.js";
import { letterGlitchAsset } from "./assets/reactbits-letter-glitch.js";
import { rotatingTextAsset } from "./assets/reactbits-rotating-text.js";
import { shinyTextAsset } from "./assets/reactbits-shiny-text.js";
import { splitFlapTextAsset } from "./assets/reactbits-split-flap-text.js";
import { springCheckAsset } from "./assets/reactbits-spring-check.js";
import { starBorderAsset } from "./assets/reactbits-star-border.js";
import { stepperAsset } from "./assets/reactbits-stepper.js";
import { strokeTextAsset } from "./assets/reactbits-stroke-text.js";
import { textLoopAsset } from "./assets/reactbits-text-loop.js";
import { tiltedCardAsset } from "./assets/reactbits-tilted-card.js";
import { trueFocusAsset } from "./assets/reactbits-true-focus.js";
import { warpTextAsset } from "./assets/reactbits-warp-text.js";
import { wavesAsset } from "./assets/reactbits-waves.js";

export const REACT_BITS_ASSETS: AssetManifest[] = [
  circularTextAsset, // reactbits-circular-text
  decryptedTextAsset, // reactbits-decrypted-text
  dockAsset, // reactbits-dock
  dotGridAsset, // reactbits-dot-grid
  electricBorderAsset, // reactbits-electric-border
  foldTextAsset, // reactbits-fold-text
  fuzzyTextAsset, // reactbits-fuzzy-text
  glareHoverAsset, // reactbits-glare-hover
  letterGlitchAsset, // reactbits-letter-glitch
  rotatingTextAsset, // reactbits-rotating-text
  shinyTextAsset, // reactbits-shiny-text
  splitFlapTextAsset, // reactbits-split-flap-text
  springCheckAsset, // reactbits-spring-check
  starBorderAsset, // reactbits-star-border
  stepperAsset, // reactbits-stepper
  strokeTextAsset, // reactbits-stroke-text
  textLoopAsset, // reactbits-text-loop
  tiltedCardAsset, // reactbits-tilted-card
  trueFocusAsset, // reactbits-true-focus
  warpTextAsset, // reactbits-warp-text
  wavesAsset, // reactbits-waves
];
