export const DESIGN_STYLES = ["claude", "openai", "notion", "linear"] as const;

export type DesignStyle = (typeof DESIGN_STYLES)[number];

export const DESIGN_STYLE_STORAGE_KEY = "zcode-design-style";

/**
 * 四套预设风格的 ramp 预览色（与 styles.css 的 --style-* 定义逐值对应）。
 * ponytail: 这是给皮肤工坊色板兜底用的只读副本——真源仍是 styles.css；改风格
 * 色值时两处要同步，后续若做"风格即皮肤"统一再收敛成单一来源。
 */
export const DESIGN_STYLE_RAMP_PREVIEW: Record<
  DesignStyle,
  { light: Record<string, string>; dark: Record<string, string> }
> = {
  claude: {
    light: {
      background: "#f5f4ed",
      backgroundDeep: "#e8e6dc",
      surface: "#faf9f5",
      sidebar: "#efece3",
      ink: "#141413",
      brand: "#c96442",
      brandStrong: "#a85034",
      accent: "#f3e6df",
    },
    dark: {
      background: "#141413",
      backgroundDeep: "#30302e",
      surface: "#232321",
      sidebar: "#10100f",
      ink: "#faf9f5",
      brand: "#d97757",
      brandStrong: "#e7b2a1",
      accent: "#3a2b26",
    },
  },
  openai: {
    light: {
      background: "#ffffff",
      backgroundDeep: "#ececec",
      surface: "#fafafa",
      sidebar: "#f5f5f5",
      ink: "#0d0d0d",
      brand: "#10a37f",
      brandStrong: "#0a7a5e",
      accent: "#e8f5f0",
    },
    dark: {
      background: "#0d0d0d",
      backgroundDeep: "#212121",
      surface: "#171717",
      sidebar: "#0a0a0a",
      ink: "#ffffff",
      brand: "#10a37f",
      brandStrong: "#8ed7c5",
      accent: "#16362e",
    },
  },
  notion: {
    light: {
      background: "#ffffff",
      backgroundDeep: "#e9e7e4",
      surface: "#f6f5f4",
      sidebar: "#f6f5f4",
      ink: "#31302e",
      brand: "#0075de",
      brandStrong: "#005bab",
      accent: "#e8f3fc",
    },
    dark: {
      background: "#191919",
      backgroundDeep: "#2f2f2f",
      surface: "#202020",
      sidebar: "#141414",
      ink: "#f6f5f4",
      brand: "#4ea7ef",
      brandStrong: "#b9dbf8",
      accent: "#1c3346",
    },
  },
  linear: {
    light: {
      background: "#f7f8f8",
      backgroundDeep: "#e6e8eb",
      surface: "#ffffff",
      sidebar: "#f3f4f5",
      ink: "#18191a",
      brand: "#5e6ad2",
      brandStrong: "#4d58b8",
      accent: "#eceefb",
    },
    dark: {
      background: "#08090a",
      backgroundDeep: "#191a1b",
      surface: "#121314",
      sidebar: "#050606",
      ink: "#f7f8f8",
      brand: "#7170ff",
      brandStrong: "#b8b9ff",
      accent: "#23243d",
    },
  },
};

export function isDesignStyle(value: string | null): value is DesignStyle {
  return DESIGN_STYLES.some((style) => style === value);
}

export function normalizeDesignStyle(value: string | null | undefined): DesignStyle {
  return isDesignStyle(value ?? null) ? (value as DesignStyle) : "claude";
}
