import { useEffect, useRef, useState } from "react";
import { useZCodeStore } from "@/store/StoreProvider.js";
import { readSkinVideoAsset } from "@/skin-engine/skinVideoAssets.js";

/**
 * 壁纸层——RootShell 的第一个子元素，垫在整个界面内容之下（absolute -z-10）。
 * 支持图片/动图（<img> 原生循环）、视频（静音循环 + 失焦暂停）与 CSS 渐变。
 * 视频本体存 IndexedDB（skinVideoAssets），这里取 Blob 转 objectURL 喂 <video>。
 * 壁纸能"看见"的前提是各区域表面被玻璃化调成半透明（skinApply 的语义色覆盖）。
 */
export function WallpaperLayer() {
  const skinLibrary = useZCodeStore((state) => state.skinLibrary);
  const activeSkinId = useZCodeStore((state) => state.activeSkinId);
  const [imageFailed, setImageFailed] = useState(false);
  const [videoSrc, setVideoSrc] = useState<string | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);

  const wallpaper = skinLibrary.find((pack) => pack.id === activeSkinId)?.wallpaper;
  const videoAssetId = wallpaper?.kind === "video" ? wallpaper.videoAssetId : undefined;

  useEffect(() => {
    setImageFailed(false);
  }, [wallpaper?.imageDataUrl, videoAssetId]);

  // 视频素材：IndexedDB 取 Blob → objectURL（卸载/换素材即回收）。查不到
  // （清过浏览器数据/换机器）→ 无源 → 视频层不渲染，皮肤其余部分照常。
  useEffect(() => {
    if (!videoAssetId) {
      setVideoSrc(null);
      return;
    }
    let disposed = false;
    let objectUrl: string | null = null;
    void readSkinVideoAsset(videoAssetId).then((blob) => {
      if (disposed || !blob) return;
      objectUrl = URL.createObjectURL(blob);
      setVideoSrc(objectUrl);
    });
    return () => {
      disposed = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
      setVideoSrc(null);
    };
  }, [videoAssetId]);

  const pauseWhenUnfocused =
    wallpaper?.kind === "video" ? wallpaper.pauseWhenUnfocused !== false : false;

  // 视频壁纸失焦自动暂停（原方案抄 dsh-wallpaper 的省电坑清单）：页面隐藏/窗口失焦
  // 暂停播放，回到前台再续上。
  useEffect(() => {
    if (!pauseWhenUnfocused) return;
    const video = videoRef.current;
    if (!video) return;
    const pause = () => video.pause();
    const resume = () => {
      if (document.visibilityState === "visible") void video.play().catch(() => undefined);
    };
    const onVisibility = () => {
      if (document.visibilityState !== "visible") pause();
      else resume();
    };
    window.addEventListener("blur", pause);
    window.addEventListener("focus", resume);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.removeEventListener("blur", pause);
      window.removeEventListener("focus", resume);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [pauseWhenUnfocused, videoSrc]);

  if (!wallpaper) return null;
  if (wallpaper.kind === "image" && (!wallpaper.imageDataUrl || imageFailed)) return null;
  if (wallpaper.kind === "gradient" && !wallpaper.gradient) return null;
  if (wallpaper.kind === "video" && !videoSrc) return null;

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
      ) : wallpaper.kind === "video" ? (
        <video
          ref={videoRef}
          src={videoSrc ?? undefined}
          autoPlay
          muted
          loop
          playsInline
          disablePictureInPicture
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
