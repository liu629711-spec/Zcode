import type { AssetManifest } from "../types.js";
import { UIVERSE_AUTO_BUTTONS } from "./uiverse-buttons.js";
import { UIVERSE_AUTO_CARDS } from "./uiverse-cards.js";
import { UIVERSE_AUTO_CHECKBOX } from "./uiverse-checkbox.js";
import { UIVERSE_AUTO_FORMS } from "./uiverse-forms.js";
import { UIVERSE_AUTO_INPUTS } from "./uiverse-inputs.js";
import { UIVERSE_AUTO_LOADERS } from "./uiverse-loaders.js";
import { UIVERSE_AUTO_NOTIFICATIONS } from "./uiverse-notifications.js";
import { UIVERSE_AUTO_PATTERNS } from "./uiverse-patterns.js";
import { UIVERSE_AUTO_RADIO_BUTTON } from "./uiverse-radio-button.js";
import { UIVERSE_AUTO_TOGGLE_SWITCH } from "./uiverse-toggle-switch.js";
import { UIVERSE_AUTO_TOOLTIPS } from "./uiverse-tooltips.js";

/**
 * UIverse 自动收录货的总袋（3,793 件，scripts/import-uiverse-galaxy.mjs 生成）。
 * 只被展厅的动态 import 消费（AssetLibrarySection），不进主包—— mentions/composer
 * 的静态 ASSET_CATALOG 不含这批货。
 */
export const BULK_AUTO_ASSETS: AssetManifest[] = [
  ...UIVERSE_AUTO_BUTTONS,
  ...UIVERSE_AUTO_CARDS,
  ...UIVERSE_AUTO_CHECKBOX,
  ...UIVERSE_AUTO_FORMS,
  ...UIVERSE_AUTO_INPUTS,
  ...UIVERSE_AUTO_LOADERS,
  ...UIVERSE_AUTO_NOTIFICATIONS,
  ...UIVERSE_AUTO_PATTERNS,
  ...UIVERSE_AUTO_RADIO_BUTTON,
  ...UIVERSE_AUTO_TOGGLE_SWITCH,
  ...UIVERSE_AUTO_TOOLTIPS,
];
