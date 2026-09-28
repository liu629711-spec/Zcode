/**
 * 沙箱小房间——素材预览的唯一容器（技术设计 §2；V3 重构为「缩放舞台」）。
 *
 * **铁律：sandbox 永远只有 `allow-scripts` 一个值。**
 * 绝不允许 allow-same-origin / allow-popups / allow-top-navigation：
 * renderer 没有 CSP（WorkflowArtifactBody.tsx 的既有教训——`allow-scripts` +
 * `allow-same-origin` 组合等价于逃逸），srcdoc 里的任何脚本都不可信。
 * 校验器 catalogCheck.assertSandboxSafe 钉住这个常量，谁改谁过不了检查。
 *
 * **V3 缩放舞台（真机反馈：展示不全/错位/偏移的根治）**：
 * demo 一律按固定设计视口（DESIGN_WIDTH×DESIGN_HEIGHT）渲染，再整体等比缩放
 * 塞进卡片的舞台框——不管 demo 内容设计得多宽多高，都完整可见、绝不裁切、
 * 绝不出滚动条、绝不错位。舞台框的纵横比恒等于设计视口（aspect-ratio 由卡片给）。
 * 这也是 beautifului.dev 等站的通用做法：预览是"整页缩略"，不是"裁剪窗口"。
 *
 * previewHtml 的收录规范（写给备货的人）：自包含、禁一切外链、禁 localStorage
 * （沙箱是无源环境，访问即抛）、动画货带 prefers-reduced-motion 降级。
 * 运行时还会再注入一段 no-scroll 样式（withNoScrollPreview）兜底——收录规范
 * 就算被未来的人违反，预览里也长不出滚动条。
 *
 * 状态全套：onLoad 前骨架态 + 骨架上的"重试"兜底（key 重挂载 iframe）；
 * previewHtml 为空串 = react 货产物没生成（忘跑 scripts/build-asset-previews.mjs），
 * 渲染错误态——这状态不该被用户看到，但必须有，防静默白屏。
 */

import { useEffect, useRef, useState } from "react";
import { ASSET_SANDBOX } from "./catalog/catalogCheck.js";

/** iframe sandbox 的唯一合法值；定义与恒等校验在 catalogCheck（此处再导出）。 */
export { ASSET_SANDBOX };

/** 设计视口：所有 demo 的"画布尺寸"。卡片舞台的纵横比必须与它一致。
 *  800×500：比 880×550 空白更少、内容占比更大（真机反馈"大片空底"）；仍是 16:10。 */
export const PREVIEW_DESIGN_WIDTH = 800;
export const PREVIEW_DESIGN_HEIGHT = 500;

/** 兜底样式：预览里永远不许出现滚动条（收录规范违反时的最后一道防线）。 */
export const NO_SCROLL_PREVIEW_STYLE =
  "<style>html,body{overflow:hidden!important;scrollbar-width:none!important}::-webkit-scrollbar{width:0!important;height:0!important;display:none!important}</style>";

/**
 * 导航拦截：收录货的 demo 里带着上游的真实外链（如 RareUI 数据表的 x.com 链接），
 * 沙箱只给 allow-scripts 时点击 <a> 会把 iframe **自己**导航走（self-navigation 不被
 * sandbox 拦），原预览整个消失——真机表现"点一下就黑了"。捕获段 preventDefault
 * 把一切链接跳转/表单提交按死在预览里；弹窗（target=_blank/window.open）本来就被
 * 无 allow-popups 的沙箱挡住，无需处理。
 */
export const NO_NAVIGATE_PREVIEW_SCRIPT =
  "<script>document.addEventListener('click',function(e){var t=e.target;if(t&&t.closest&&t.closest('a[href]')){e.preventDefault();e.stopPropagation();}},true);document.addEventListener('submit',function(e){e.preventDefault();},true);</script>";

/** 把兜底样式与导航拦截注进预览 HTML（有 head 插 head，否则前置——避免破坏 doctype 触发怪异模式）。 */
export function withNoScrollPreview(html: string): string {
  const headMatch = /<head[^>]*>/i.exec(html);
  const injected = NO_SCROLL_PREVIEW_STYLE + NO_NAVIGATE_PREVIEW_SCRIPT;
  return headMatch
    ? html.replace(headMatch[0], headMatch[0] + injected)
    : injected + html;
}

interface AssetPreviewFrameProps {
  /** 自包含预览 HTML（catalogCheck 保证无外链/无 localStorage） */
  previewHtml: string;
  /** 无障碍标题（iframe 必带） */
  title: string;
}

export function AssetPreviewFrame({ previewHtml, title }: AssetPreviewFrameProps) {
  const [attempt, setAttempt] = useState(0);
  const [loaded, setLoaded] = useState(false);
  const stageRef = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(0);

  // 换了预览内容（如详情弹层切货）时回到骨架态，别拿旧 iframe 的 loaded 硬撑
  useEffect(() => {
    setLoaded(false);
  }, [previewHtml]);

  // 缩放舞台：量舞台框实际宽度，得出缩放系数（设计视口 → 舞台框）
  useEffect(() => {
    const stage = stageRef.current;
    if (!stage) return;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? 0;
      if (width > 0) setScale(width / PREVIEW_DESIGN_WIDTH);
    });
    observer.observe(stage);
    return () => observer.disconnect();
  }, []);

  const retry = () => {
    setLoaded(false);
    setAttempt((n) => n + 1); // key 重挂载：iframe 连同其内崩溃的沙箱世界整体重建
  };

  if (previewHtml.trim() === "") {
    return (
      <div
        role="alert"
        className="flex h-full w-full flex-col items-center justify-center gap-1 rounded-lg border border-card-border bg-background p-4 text-center"
      >
        <p className="text-sm text-foreground">预览缺失，请先运行生成脚本</p>
        <p className="text-xs text-muted-foreground">
          node scripts/build-asset-previews.mjs（react 货预编译产物未生成）
        </p>
      </div>
    );
  }

  return (
    <div ref={stageRef} className="relative h-full w-full overflow-hidden">
      <iframe
        key={attempt}
        className="absolute top-0 left-0 border-0"
        style={{
          width: PREVIEW_DESIGN_WIDTH,
          height: PREVIEW_DESIGN_HEIGHT,
          transform: scale > 0 ? `scale(${scale})` : undefined,
          transformOrigin: "top left",
          visibility: scale > 0 ? undefined : "hidden",
        }}
        sandbox={ASSET_SANDBOX}
        scrolling="no"
        srcDoc={withNoScrollPreview(previewHtml)}
        title={title}
        onLoad={() => setLoaded(true)}
      />
      {!loaded && (
        <div
          role="status"
          className="absolute inset-0 flex flex-col gap-2 rounded-lg border border-card-border bg-background p-3"
        >
          <span className="sr-only">预览加载中</span>
          <span className="motion-safe:animate-pulse h-1/2 rounded-md bg-surface" />
          <span className="motion-safe:animate-pulse h-5 w-3/4 rounded-md bg-surface" />
          <span className="motion-safe:animate-pulse h-5 w-1/2 rounded-md bg-surface" />
          <button
            type="button"
            onClick={retry}
            className="mt-auto self-start rounded-md border border-card-border px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
          >
            预览卡住了？重试
          </button>
        </div>
      )}
    </div>
  );
}
