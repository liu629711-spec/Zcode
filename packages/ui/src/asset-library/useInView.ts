/**
 * 元素是否进视口（技术设计 §10 V2-1）。
 *
 * 照 pptx-preview-viewer.tsx 缩略图的 IntersectionObserver 模式封装：
 * rootMargin 提前判定，进视口（含缓冲带）true、离远 false，卸载时 disconnect。
 * 无 IntersectionObserver 的环境（jsdom 等）视作始终可见。
 * 素材库用它 gate iframe 挂载（V4.3 起传大缓冲带如 "1200px 0px"：缓冲带内挂载、
 * 离远卸载，压住活跃 WebGL 上下文数；缓冲带足够宽，重挂发生在可见之前）。
 */
import { useEffect, useRef, useState } from "react";

export function useInView<T extends HTMLElement>(rootMargin = "150px 0px"): {
  ref: React.RefObject<T | null>;
  inView: boolean;
} {
  const ref = useRef<T | null>(null);
  const [inView, setInView] = useState(false);

  useEffect(() => {
    const element = ref.current;
    if (!element || typeof IntersectionObserver === "undefined") {
      setInView(true);
      return;
    }
    const observer = new IntersectionObserver(
      ([entry]) => setInView(entry?.isIntersecting ?? false),
      { rootMargin },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [rootMargin]);

  return { ref, inView };
}
