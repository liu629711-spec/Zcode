import type { SkinGlass, SkinPack, SkinTokenSlot } from "./skinSchema.js";

/**
 * 皮肤应用器——把皮肤包翻译成一段 CSS 变量覆盖，注入 <style id="zcode-skin-overrides">。
 *
 * 机制：styles.css 的预设 ramp 定义 --style-{light,dark}-<slot>，主题块消费它们派生
 * 全套 --color-*。皮肤覆盖 ramp 槽位后，衍生色（边框/悬停/选中）自动跟随，无需逐个改。
 * 注入元素挂在 head 末尾，:root[data-zcode-skin] 与预设块同特异性靠后胜出；ramp 覆盖
 * 附加 !important——皮肤覆盖层是用户明示意图上的最终权威，这是全仓唯一豁免点。
 * focus-border 槽位不存在（皮肤包 schema 层已禁），这里也不生成。
 */

const SKIN_STYLE_ELEMENT_ID = "zcode-skin-overrides";

/** 可读性保底：玻璃化后表面最低不透明度（文字永远画在表面上） */
export const MIN_SURFACE_OPACITY = 0.5;

/** 有壁纸但皮肤包没写 glass 时的默认融入度——"有壁纸必有玻璃"，否则壁纸被不透明表面全挡住 */
export const DEFAULT_WALLPAPER_GLASS: SkinGlass = {
  enabled: true,
  blend: 0.35,
  sidebar: 0.35,
  panel: 0.35,
  composer: 0.35,
};

/** 槽位名 → ramp 变量尾段（--style-{mode}-<尾段>） */
const SLOT_VAR_SUFFIX: Record<SkinTokenSlot, string> = {
  background: "background",
  backgroundDeep: "background-deep",
  surface: "surface",
  sidebar: "sidebar",
  ink: "ink",
  brand: "brand",
  brandStrong: "brand-strong",
  accent: "accent",
};

/**
 * 玻璃化参数归一化：壁纸在场则 glass 必然生效（缺省用默认融入度）。
 * 引擎与调参 UI 共用这份口径，避免"设了壁纸却看不见"的死状态。
 */
export function resolveEffectiveGlass(skin: SkinPack): SkinGlass | undefined {
  if (skin.glass?.enabled) return skin.glass;
  if (skin.wallpaper) return DEFAULT_WALLPAPER_GLASS;
  return undefined;
}

function keepOpacityPercent(blend: number, regionMultiplier: number): number {
  const opacity = Math.max(1 - blend * regionMultiplier, MIN_SURFACE_OPACITY);
  return Math.round(opacity * 100);
}

function rampOverrideLines(skin: SkinPack): string[] {
  const lines: string[] = [];
  const modes: Array<"light" | "dark"> = ["light", "dark"];
  for (const mode of modes) {
    const overrides = skin[mode];
    if (!overrides) continue;
    for (const [slot, color] of Object.entries(overrides)) {
      lines.push(`  --style-${mode}-${SLOT_VAR_SUFFIX[slot as SkinTokenSlot]}: ${color} !important;`);
    }
  }
  return lines;
}

function glassBlockLines(skin: SkinPack, glass: SkinGlass): string[] {
  const lines: string[] = [];
  const modes: Array<"light" | "dark"> = ["light", "dark"];
  for (const mode of modes) {
    const background = `var(--style-${mode}-background)`;
    const backgroundDeep = `var(--style-${mode}-background-deep)`;
    const surface = `var(--style-${mode}-surface)`;
    const sidebar = `var(--style-${mode}-sidebar)`;
    const bgPercent = keepOpacityPercent(glass.blend, 1);
    const sidebarPercent = keepOpacityPercent(glass.blend, glass.sidebar);
    const panelPercent = keepOpacityPercent(glass.blend, glass.panel);
    const composerPercent = keepOpacityPercent(glass.blend, glass.composer);
    lines.push(
      `html.theme-zai-${mode} {`,
      `  --color-background: color-mix(in oklab, ${background} ${bgPercent}%, transparent);`,
      `  --color-background-win-alt: color-mix(in oklab, ${backgroundDeep} ${bgPercent}%, transparent);`,
      `  --color-sidebar: color-mix(in oklab, ${sidebar} ${sidebarPercent}%, transparent);`,
      `  --color-panel: color-mix(in oklab, ${surface} ${panelPercent}%, transparent);`,
      `  --color-header: color-mix(in oklab, ${surface} ${panelPercent}%, transparent);`,
      `  --color-card: color-mix(in oklab, ${surface} ${panelPercent}%, transparent);`,
      `  --color-popover: color-mix(in oklab, ${surface} ${panelPercent}%, transparent);`,
      `  --color-input: color-mix(in oklab, ${surface} ${composerPercent}%, transparent);`,
      // 工作区纯净红线：终端底色钉死不透明，壁纸不许干扰代码阅读
      `  --color-terminal-bg: ${background};`,
      `}`,
    );
  }
  return lines;
}

/** 纯函数：皮肤包 → CSS 文本。无覆盖无玻璃无壁纸时返回空串。 */
export function buildSkinCss(skin: SkinPack): string {
  const rampLines = rampOverrideLines(skin);
  const glass = resolveEffectiveGlass(skin);
  const blocks: string[] = [];
  if (rampLines.length > 0) {
    blocks.push([":root[data-zcode-skin] {", ...rampLines, "}"].join("\n"));
  }
  if (glass) {
    blocks.push(glassBlockLines(skin, glass).join("\n"));
  }
  return blocks.join("\n\n");
}

function ensureStyleElement(): HTMLStyleElement | null {
  if (typeof document === "undefined") return null;
  const existing = document.getElementById(SKIN_STYLE_ELEMENT_ID);
  if (existing instanceof HTMLStyleElement) return existing;
  existing?.remove();
  const created = document.createElement("style") as HTMLStyleElement;
  created.id = SKIN_STYLE_ELEMENT_ID;
  document.head.appendChild(created);
  return created;
}

/**
 * 把皮肤应用到文档（幂等：单元素重建文本，不累积）。
 * 传 null = 卸载皮肤（回预设），移除元素与标记属性。
 */
export function applySkinToDocument(skin: SkinPack | null): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  const cssText = skin ? buildSkinCss(skin) : "";
  if (!skin || cssText === "") {
    document.getElementById(SKIN_STYLE_ELEMENT_ID)?.remove();
    delete root.dataset.zcodeSkin;
    return;
  }
  root.dataset.zcodeSkin = "1";
  const element = ensureStyleElement();
  if (element) {
    element.textContent = cssText;
  }
}
