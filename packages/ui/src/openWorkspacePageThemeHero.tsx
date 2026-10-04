import type { ReactNode } from "react";
import { cn } from "@/components/lib/utils.js";
import { useZCodeStore } from "@/store/StoreProvider.js";
import { resolveTheme } from "@/useTheme.js";
import type { Theme } from "@/useTheme.js";

interface ThemeHeroPalette {
  panel: string;
  meshBase: string;
  meshLight: string;
  glowPrimary: string;
  glowSecondary: string;
  heading: string;
  description: string;
}

function readThemeHex(name: string, fallback: string): string {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return /^#[0-9a-fA-F]{6}$/.test(value) ? value : fallback;
}

function getThemeHeroPalette(theme: Theme): ThemeHeroPalette {
  // 欢迎页配色从当前主题的语义变量实时读取：跟随设置里选的设计风格与明暗模式，
  // 不再按主题硬编码渐变（旧硬编码曾与所选风格冲突）。
  const dark = theme === "dark" || theme === "zai-dark";
  const background = readThemeHex("--color-background", dark ? "#141413" : "#f5f4ed");
  const brand = readThemeHex("--color-brand", dark ? "#d97757" : "#c96442");
  return {
    meshBase: background,
    meshLight: brand,
    panel: dark
      ? "bg-[linear-gradient(180deg,var(--style-dark-background)_0%,var(--style-dark-surface)_46%,var(--style-dark-accent)_100%)] before:absolute before:inset-0 before:content-[''] before:bg-[radial-gradient(circle_at_18%_18%,color-mix(in_oklab,var(--color-brand)_16%,transparent),transparent_24%),radial-gradient(circle_at_82%_12%,color-mix(in_oklab,var(--color-brand)_10%,transparent),transparent_26%),radial-gradient(circle_at_66%_84%,color-mix(in_oklab,var(--color-brand)_12%,transparent),transparent_28%)]"
      : "bg-[linear-gradient(180deg,var(--style-light-surface)_0%,var(--style-light-background)_46%,var(--style-light-accent)_100%)] before:absolute before:inset-0 before:content-[''] before:bg-[radial-gradient(circle_at_18%_20%,rgba(255,255,255,0.72),transparent_24%),radial-gradient(circle_at_82%_14%,color-mix(in_oklab,var(--color-brand)_14%,transparent),transparent_26%),radial-gradient(circle_at_70%_84%,color-mix(in_oklab,var(--color-brand)_10%,transparent),transparent_30%)]",
    glowPrimary: dark
      ? "bg-[color-mix(in_oklab,var(--color-brand)_22%,transparent)] mix-blend-screen"
      : "bg-[color-mix(in_oklab,var(--color-brand)_28%,transparent)] mix-blend-multiply",
    glowSecondary: dark
      ? "bg-[color-mix(in_oklab,var(--color-brand)_12%,transparent)] mix-blend-screen"
      : "bg-[color-mix(in_oklab,var(--color-brand)_12%,transparent)] mix-blend-multiply",
    heading: "text-[var(--color-foreground)]",
    description: "text-[color-mix(in_oklab,var(--color-foreground)_64%,transparent)]",
  };
}

export function useResolvedThemeHeroPalette(): ThemeHeroPalette {
  const theme = useZCodeStore((state) => state.theme);
  const resolvedTheme =
    theme === "system" ? (resolveTheme(theme) === "dark" ? "dark" : "light") : theme;

  return getThemeHeroPalette(resolvedTheme);
}

export function ThemeHeroVisual(props: {
  className?: string;
  contentClassName?: string;
  children?: ReactNode;
}) {
  const palette = useResolvedThemeHeroPalette();

  return (
    <div className={cn("relative overflow-hidden", palette.panel, props.className)}>
      <div
        className={cn(
          "pointer-events-none absolute left-[-12%] top-[18%] h-[42rem] w-[42rem] rounded-full blur-3xl",
          palette.glowPrimary,
        )}
      />
      <div
        className={cn(
          "pointer-events-none absolute right-[-18%] bottom-[-14%] h-[36rem] w-[36rem] rounded-full blur-3xl",
          palette.glowSecondary,
        )}
      />
      {props.children ? (
        <div className={cn("relative z-10", props.contentClassName)}>{props.children}</div>
      ) : null}
    </div>
  );
}
