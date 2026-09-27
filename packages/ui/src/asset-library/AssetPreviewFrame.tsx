/**
 * 沙箱小房间——素材预览的唯一容器（技术设计 §2）。
 *
 * **铁律：sandbox 永远只有 `allow-scripts` 一个值。**
 * 绝不允许 allow-same-origin / allow-popups / allow-top-navigation：
 * renderer 没有 CSP（WorkflowArtifactBody.tsx 的既有教训——`allow-scripts` +
 * `allow-same-origin` 组合等价于逃逸），srcdoc 里的任何脚本都不可信。
 * 校验器 catalogCheck.assertSandboxSafe 钉住这个常量，谁改谁过不了检查。
 *
 * previewHtml 的收录规范（写给备货的人）：自包含、禁一切外链、禁 localStorage
 * （沙箱是无源环境，访问即抛）、动画货带 prefers-reduced-motion 降级。
 *
 * 状态全套：onLoad 前骨架态（token 对齐 AutomationTemplateSkeletonGrid）+
 * 骨架上的"重试"兜底（key 重挂载 iframe）；previewHtml 为空串 = react 货产物
 * 没生成（忘跑 scripts/build-asset-previews.mjs），渲染错误态——这状态不该被
 * 用户看到，但必须有，防静默白屏。
 */

import { useEffect, useState } from "react";
import { ASSET_SANDBOX } from "./catalog/catalogCheck.js";

/** iframe sandbox 的唯一合法值；定义与恒等校验在 catalogCheck（此处再导出）。 */
export { ASSET_SANDBOX };

interface AssetPreviewFrameProps {
  /** 自包含预览 HTML（catalogCheck 保证无外链/无 localStorage） */
  previewHtml: string;
  /** 无障碍标题（iframe 必带） */
  title: string;
}

export function AssetPreviewFrame({ previewHtml, title }: AssetPreviewFrameProps) {
  const [attempt, setAttempt] = useState(0);
  const [loaded, setLoaded] = useState(false);

  // 换了预览内容（如详情弹层切货）时回到骨架态，别拿旧 iframe 的 loaded 硬撑
  useEffect(() => {
    setLoaded(false);
  }, [previewHtml]);

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
    <div className="relative h-full w-full">
      <iframe
        key={attempt}
        className="block h-full w-full rounded-lg border-0"
        sandbox={ASSET_SANDBOX}
        srcDoc={previewHtml}
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
