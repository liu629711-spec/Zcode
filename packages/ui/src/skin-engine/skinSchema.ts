import { DESIGN_STYLES, type DesignStyle } from "../themeStyles.js";

/**
 * 皮肤包（SkinPack）格式与校验——换肤引擎的进货验收单。
 *
 * 设计约定（docs/ZCode-主题壁纸系统-调研与方案.md v2.1）：
 * - 皮肤只覆盖 --style-* 九槽 ramp 中的 8 个颜色槽位；focus-border 故意不提供
 *   （彩色聚焦边框被用户明确否决，机制级禁止，皮肤包做不了这颗雷）。
 * - 校验手写（packages/ui 不依赖 zod），风格与 themeStyles.ts 一致。
 * - 未知字段剥离（前向兼容：新版本客户端写的字段，旧版本引擎忽略不报错）。
 * - formatVersion 不匹配直接拒绝——宁可拒装，不可带病上屏。
 */

export const SKIN_FORMAT_VERSION = 1;

/** 皮肤可覆盖的 token 槽位。对应 styles.css 的 --style-{light,dark}-<slot> 变量。 */
export const SKIN_TOKEN_SLOTS = [
  "background",
  "backgroundDeep",
  "surface",
  "sidebar",
  "ink",
  "brand",
  "brandStrong",
  "accent",
] as const;

export type SkinTokenSlot = (typeof SKIN_TOKEN_SLOTS)[number];

export type SkinTokenOverrides = Partial<Record<SkinTokenSlot, string>>;

export interface SkinWallpaper {
  kind: "image" | "gradient";
  /** 仅 kind=image：data:image/* 或 http(s) URL，内嵌素材上限见 MAX_SKIN_IMAGE_CHARS */
  imageDataUrl?: string;
  /** 仅 kind=gradient：CSS 渐变函数字符串 */
  gradient?: string;
  /** 壁纸层自身不透明度 */
  opacity: number;
  /** 壁纸模糊半径 px */
  blurPx: number;
  /** 暗化遮罩强度（压暗壁纸保证前景可读） */
  dim: number;
}

export interface SkinGlass {
  enabled: boolean;
  /** 融入度：表面让壁纸透出来的整体强度 0..1 */
  blend: number;
  /** 区域倍率 0..1，缺省跟随 blend */
  sidebar: number;
  panel: number;
  composer: number;
}

export interface SkinPack {
  formatVersion: typeof SKIN_FORMAT_VERSION;
  /** 稳定 id：库内去重/激活引用/分享链接都用它 */
  id: string;
  name: string;
  author?: string;
  /** 继承哪套预设风格的 ramp 作为底 */
  baseStyle: DesignStyle;
  light?: SkinTokenOverrides;
  dark?: SkinTokenOverrides;
  wallpaper?: SkinWallpaper;
  glass?: SkinGlass;
}

export type SkinValidateResult =
  | { ok: true; skin: SkinPack }
  | { ok: false; errors: string[] };

/** data URL 壁纸上限（字符）：localStorage 单键约 5MB，留余量给库里其他皮肤 */
export const MAX_SKIN_IMAGE_CHARS = 4_000_000;

const HEX_COLOR_RE = /^#(?:[0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const SKIN_ID_RE = /^[a-z0-9][a-z0-9_-]{0,63}$/i;
const GRADIENT_FUNCTION_RE =
  /^(?:linear|radial|conic)-gradient\(/i;
const IMAGE_SOURCE_RE = /^(?:data:image\/(?:png|jpeg|webp|gif|avif|svg\+xml);|https?:\/\/)/;

/**
 * 渐变串加固：只当"纯色渐变"用——括号配平、禁分号（防逃逸出声明块）、禁 url(
 * （渐变不该引用外部资源，也顺手掐掉远程像素跟踪）。
 */
function isSafeGradient(value: string): boolean {
  if (value.includes(";") || /url\(/i.test(value)) return false;
  let depth = 0;
  for (const char of value) {
    if (char === "(") depth += 1;
    else if (char === ")") {
      depth -= 1;
      if (depth < 0) return false;
    }
  }
  return depth === 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function pickHexColor(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return HEX_COLOR_RE.test(trimmed) ? trimmed.toLowerCase() : undefined;
}

/** 区域/壁纸数值参数：夹到闭区间，越界算非法数据还是悄悄夹住？——校验从严（报错），运行时夹取另行 */
function validateBoundedNumber(
  value: unknown,
  min: number,
  max: number,
  label: string,
  errors: string[],
): number | undefined {
  if (!isFiniteNumber(value)) {
    errors.push(`${label} 必须是数字`);
    return undefined;
  }
  if (value < min || value > max) {
    errors.push(`${label} 必须在 ${min} ~ ${max} 之间`);
    return undefined;
  }
  return value;
}

function validateTokenOverrides(
  value: unknown,
  label: string,
  errors: string[],
): SkinTokenOverrides | undefined {
  if (value === undefined) return undefined;
  if (!isRecord(value)) {
    errors.push(`${label} 必须是对象`);
    return undefined;
  }
  const overrides: SkinTokenOverrides = {};
  for (const [key, raw] of Object.entries(value)) {
    if (!(SKIN_TOKEN_SLOTS as readonly string[]).includes(key)) {
      errors.push(`${label}.${key} 不是可覆盖的槽位`);
      continue;
    }
    const hex = pickHexColor(raw);
    if (!hex) {
      errors.push(`${label}.${key} 必须是 #RGB/#RRGGBB/#RRGGBBAA 色值`);
      continue;
    }
    overrides[key as SkinTokenSlot] = hex;
  }
  return overrides;
}

function validateWallpaper(value: unknown, errors: string[]): SkinWallpaper | undefined {
  if (value === undefined) return undefined;
  if (!isRecord(value)) {
    errors.push("wallpaper 必须是对象");
    return undefined;
  }
  const kind = value.kind;
  if (kind !== "image" && kind !== "gradient") {
    errors.push('wallpaper.kind 必须是 "image" 或 "gradient"');
    return undefined;
  }
  const wallpaper: SkinWallpaper = {
    kind,
    opacity: 1,
    blurPx: 0,
    dim: 0,
  };
  if (kind === "image") {
    const source = value.imageDataUrl;
    if (typeof source !== "string" || !IMAGE_SOURCE_RE.test(source.trim())) {
      errors.push("wallpaper.imageDataUrl 必须是 data:image/* 或 http(s) 图片地址");
      return undefined;
    }
    if (source.length > MAX_SKIN_IMAGE_CHARS) {
      errors.push(`wallpaper.imageDataUrl 超过 ${MAX_SKIN_IMAGE_CHARS} 字符上限`);
      return undefined;
    }
    wallpaper.imageDataUrl = source.trim();
  } else {
    const gradient = value.gradient;
    if (
      typeof gradient !== "string" ||
      !GRADIENT_FUNCTION_RE.test(gradient.trim()) ||
      gradient.length > 2000 ||
      !isSafeGradient(gradient.trim())
    ) {
      errors.push("wallpaper.gradient 必须是 linear/radial/conic-gradient 渐变字符串");
      return undefined;
    }
    wallpaper.gradient = gradient.trim();
  }
  const opacity = validateBoundedNumber(value.opacity, 0.15, 1, "wallpaper.opacity", errors);
  if (opacity === undefined) return undefined;
  const blurPx = validateBoundedNumber(value.blurPx, 0, 40, "wallpaper.blurPx", errors);
  if (blurPx === undefined) return undefined;
  const dim = validateBoundedNumber(value.dim, 0, 0.85, "wallpaper.dim", errors);
  if (dim === undefined) return undefined;
  wallpaper.opacity = opacity;
  wallpaper.blurPx = blurPx;
  wallpaper.dim = dim;
  return wallpaper;
}

function validateGlass(value: unknown, errors: string[]): SkinGlass | undefined {
  if (value === undefined) return undefined;
  if (!isRecord(value)) {
    errors.push("glass 必须是对象");
    return undefined;
  }
  if (typeof value.enabled !== "boolean") {
    errors.push("glass.enabled 必须是布尔值");
    return undefined;
  }
  const blend = validateBoundedNumber(value.blend, 0, 1, "glass.blend", errors);
  if (blend === undefined) return undefined;
  const sidebar = value.sidebar === undefined ? blend : value.sidebar;
  const panel = value.panel === undefined ? blend : value.panel;
  const composer = value.composer === undefined ? blend : value.composer;
  const sidebarV = validateBoundedNumber(sidebar, 0, 1, "glass.sidebar", errors);
  const panelV = validateBoundedNumber(panel, 0, 1, "glass.panel", errors);
  const composerV = validateBoundedNumber(composer, 0, 1, "glass.composer", errors);
  if (sidebarV === undefined || panelV === undefined || composerV === undefined) return undefined;
  return { enabled: value.enabled, blend, sidebar: sidebarV, panel: panelV, composer: composerV };
}

/** 校验并归一化一个裸皮肤包（导入/分享链接/localStorage 回读共用这条门）。 */
export function validateSkin(raw: unknown): SkinValidateResult {
  if (!isRecord(raw)) {
    return { ok: false, errors: ["皮肤包必须是 JSON 对象"] };
  }
  const errors: string[] = [];
  if (raw.formatVersion !== SKIN_FORMAT_VERSION) {
    return {
      ok: false,
      errors: [
        `formatVersion 必须是 ${SKIN_FORMAT_VERSION}（收到：${String(raw.formatVersion)}）`,
      ],
    };
  }
  const id = typeof raw.id === "string" ? raw.id.trim() : "";
  if (!SKIN_ID_RE.test(id)) {
    errors.push("id 必须是 1~64 位字母/数字/-/_ 且以字母数字开头");
  }
  const name = typeof raw.name === "string" ? raw.name.trim() : "";
  if (name.length < 1 || name.length > 60) {
    errors.push("name 必须是 1~60 个字符");
  }
  const author = typeof raw.author === "string" ? raw.author.trim().slice(0, 60) : undefined;

  let baseStyle: DesignStyle = "claude";
  if (raw.baseStyle !== undefined) {
    if (typeof raw.baseStyle === "string" && (DESIGN_STYLES as readonly string[]).includes(raw.baseStyle)) {
      baseStyle = raw.baseStyle as DesignStyle;
    } else {
      errors.push(`baseStyle 必须是 ${DESIGN_STYLES.join(" / ")} 之一`);
    }
  }

  const light = validateTokenOverrides(raw.light, "light", errors);
  const dark = validateTokenOverrides(raw.dark, "dark", errors);
  const wallpaper = validateWallpaper(raw.wallpaper, errors);
  const glass = validateGlass(raw.glass, errors);

  if (errors.length > 0) {
    return { ok: false, errors };
  }
  const skin: SkinPack = {
    formatVersion: SKIN_FORMAT_VERSION,
    id,
    name,
    ...(author ? { author } : {}),
    baseStyle,
    ...(light && Object.keys(light).length > 0 ? { light } : {}),
    ...(dark && Object.keys(dark).length > 0 ? { dark } : {}),
    ...(wallpaper ? { wallpaper } : {}),
    ...(glass ? { glass } : {}),
  };
  return { ok: true, skin };
}

/** 皮肤包序列化（导入导出/分享统一走这里，保证字段收敛）。 */
export function serializeSkin(skin: SkinPack): string {
  return JSON.stringify(skin, null, 2);
}

/** 以某套预设风格为底，起一个空皮肤（"从当前风格创建皮肤"用）。 */
export function createSkinFromStyle(
  baseStyle: DesignStyle,
  id: string,
  name: string,
): SkinPack {
  return {
    formatVersion: SKIN_FORMAT_VERSION,
    id,
    name,
    baseStyle,
  };
}
