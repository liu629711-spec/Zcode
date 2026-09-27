/**
 * 元素是否进视口（技术设计 §10 V2-1）。
 *
 * 照 pptx-preview-viewer.tsx 缩略图的 IntersectionObserver 模式封装：
 * rootMargin 提前 200px 判定，进视口 true、出视口 false，卸载时 disconnect。
 * 无 IntersectionObserver 的环境（jsdom 等）视作始终可见。
 * 素材库用它 gate iframe 挂载：任何时刻同屏沙箱数 = 视口内卡片数（±200px）。
 */
import { useEffect, useRef, useState } from "react";

export function useInView<T extends HTMLElement>(rootMargin = "200px 0px"): {
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
