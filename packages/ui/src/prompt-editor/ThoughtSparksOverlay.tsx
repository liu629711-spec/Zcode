import { useEffect, useRef } from "react";

/*
 * 思考深度最高档的输入壳特效（自实现 React Bits PromptBar 的 max-effort 行为）：
 * 壳内底部泛起品牌色光晕，火花从底部上浮，打字越快越活跃。
 * 火花颜色读 --color-composer-spark，回落到 --color-brand，因此四套设计风格各自成色；
 * prefers-reduced-motion 下只保留光晕，不跑粒子动画。
 */

type Spark = {
  x: number;
  y: number;
  r: number;
  vy: number;
  sway: number;
  phase: number;
  life: number;
  span: number;
};

const MAX_SPARKS = 30;
const SPAWN_INTERVAL_S = 0.14;

export function ThoughtSparksOverlay({ active }: { active: boolean }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const typing = useRef({ energy: 0, strokes: 0 });

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !active) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const computed = getComputedStyle(canvas);
    const sparkColor =
      computed.getPropertyValue("--color-composer-spark").trim() ||
      computed.getPropertyValue("--color-brand").trim() ||
      "#b39dff";

    const parts: Spark[] = [];
    let raf = 0;
    let last = performance.now();
    let due = 0;
    let speed = 1;
    let pulse = 0;
    let w = 0;
    let h = 0;

    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      w = rect.width;
      h = rect.height;
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const spawn = () => {
      parts.push({
        x: Math.random() * w,
        y: h + 3,
        r: 0.9 + Math.random() * 1.1,
        vy: -(7 + Math.random() * 9),
        sway: (Math.random() - 0.5) * 10,
        phase: Math.random() * Math.PI * 2,
        life: 0,
        span: 2.4 + Math.random() * 2.4,
      });
    };

    // 打字能量：编辑器的原生 input 事件冒泡到 document，捕获一次即可，粒子随打字加速增亮。
    const onInput = () => {
      typing.current.energy = Math.min(1.6, typing.current.energy + 0.22);
      typing.current.strokes = Math.min(4, typing.current.strokes + 1);
    };
    document.addEventListener("input", onInput, true);

    const tick = (now: number) => {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      const typed = typing.current;
      typed.energy *= Math.exp(-dt / 0.8);
      pulse *= Math.exp(-dt / 0.16);
      if (typed.strokes > 0) {
        typed.strokes = 0;
        pulse = 1;
      }
      speed += (1 + typed.energy * 6 - speed) * (1 - Math.exp(-dt / 0.15));
      due += dt;
      while (due > SPAWN_INTERVAL_S) {
        due -= SPAWN_INTERVAL_S;
        if (parts.length < MAX_SPARKS) spawn();
      }
      ctx.clearRect(0, 0, w, h);
      ctx.fillStyle = sparkColor;
      ctx.shadowColor = sparkColor;
      ctx.shadowBlur = 6 + typed.energy * 10 + pulse * 6;
      for (let i = parts.length - 1; i >= 0; i -= 1) {
        const p = parts[i];
        if (!p) {
          parts.splice(i, 1);
          continue;
        }
        p.life += dt;
        if (p.life > p.span) {
          parts.splice(i, 1);
          continue;
        }
        const k = p.life / p.span;
        const twinkle = 0.7 + 0.3 * Math.sin((now / 160) * (1 + typed.energy) + p.phase);
        p.y += p.vy * dt * speed;
        if (p.y < -4) {
          p.y = h + 3;
          p.x = Math.random() * w;
        }
        const edge = Math.min(1, Math.max(0, p.y / 14), Math.max(0, (h - p.y) / 14));
        ctx.globalAlpha =
          Math.min(1, Math.sin(k * Math.PI) * (0.9 + typed.energy * 0.25) * twinkle) * edge;
        ctx.beginPath();
        ctx.arc(
          p.x + Math.sin((now / 900) * (1 + typed.energy * 0.8) + p.phase) * p.sway,
          p.y,
          p.r * twinkle * (1 + typed.energy * 0.35),
          0,
          Math.PI * 2,
        );
        ctx.fill();
      }
      raf = requestAnimationFrame(tick);
    };

    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    raf = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
      document.removeEventListener("input", onInput, true);
      ctx.clearRect(0, 0, w, h);
    };
  }, [active]);

  return (
    <div
      aria-hidden
      data-active={active ? "true" : "false"}
      className="thought-sparks pointer-events-none absolute inset-0 -z-10 overflow-hidden rounded-[inherit]"
    >
      <span className="thought-sparks-wash" />
      {active ? <canvas ref={canvasRef} className="absolute inset-0 h-full w-full" /> : null}
    </div>
  );
}
