import { useEffect, useState } from "react";
import { useZCodeStore } from "@/store/StoreProvider.js";

/**
 * 壁纸层——RootShell 的第一个子元素，垫在整个界面内容之下（absolute -z-10）。
 * 支持图片/动图（<img> 原生循环）与 CSS 渐变；视频/网页壁纸在批 2。
 * 壁纸能"看见"的前提是各区域表面被玻璃化调成半透明（skinApply 的语义色覆盖）。
 */
export function WallpaperLayer() {
  const skinLibrary = useZCodeStore((state) => state.skinLibrary);
  const activeSkinId = useZCodeStore((state) => state.activeSkinId);
  const [imageFailed, setImageFailed] = useState(false);

  const wallpaper = skinLibrary.find((pack) => pack.id === activeSkinId)?.wallpaper;

  useEffect(() => {
    setImageFailed(false);
  }, [wallpaper?.imageDataUrl]);

  if (!wallpaper) return null;
  if (wallpaper.kind === "image" && (!wallpaper.imageDataUrl || imageFailed)) return null;
  if (wallpaper.kind === "gradient" && !wallpaper.gradient) return null;

  // 模糊会把边缘磨出透明晕边，放大一点裁掉；blur 值来自皮肤包，静态无动画
  const decoration = {
    opacity: wallpaper.opacity,
    filter: wallpaper.blurPx > 0 ? `blur(${wallpaper.blurPx}px)` : undefined,
    transform: wallpaper.blurPx > 0 ? "scale(1.08)" : undefined,
  };

  return (
    <div
      aria-hidden
      className="pointer-events-none absolute inset-0 -z-10 select-none overflow-hidden"
      data-testid="skin-wallpaper-layer"
    >
      {wallpaper.kind === "image" ? (
        <img
          src={wallpaper.imageDataUrl}
          alt=""
          draggable={false}
          className="absolute inset-0 size-full object-cover"
          style={decoration}
          onError={() => setImageFailed(true)}
        />
      ) : (
        <div className="absolute inset-0" style={{ background: wallpaper.gradient, ...decoration }} />
      )}
      {wallpaper.dim > 0 ? (
        <div className="absolute inset-0 bg-black" style={{ opacity: wallpaper.dim }} />
      ) : null}
    </div>
  );
}
