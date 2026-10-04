export const DESIGN_STYLES = ["claude", "openai", "notion", "linear"] as const;

export type DesignStyle = (typeof DESIGN_STYLES)[number];

export const DESIGN_STYLE_STORAGE_KEY = "zcode-design-style";

export function isDesignStyle(value: string | null): value is DesignStyle {
  return DESIGN_STYLES.some((style) => style === value);
}

export function normalizeDesignStyle(value: string | null | undefined): DesignStyle {
  return isDesignStyle(value ?? null) ? (value as DesignStyle) : "claude";
}
