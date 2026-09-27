/**
 * 素材大演示卡（技术设计 §10 V2-1，beautifului.dev 式画廊的单件载体）。
 *
 * 序号+分类徽标+标题+一句话+右上角常显悬浮操作组（代码/复制/递活，ghost hover 提亮）；
 * 演示区 h-[380px] 内嵌真 AssetPreviewFrame，由 useInView 现挂现卸（出视口回落骨架），
 * 保证任何时刻同屏沙箱数 = 视口内卡片数（±200px rootMargin），不许 30 个同挂。
 * 点 `</>` 在卡片下方内联展开代码面板（ai-elements CodeBlock/shiki 渲染+自带
 * CodeBlockCopyButton；多文件用其自带 selector 切换）；prompt 类货（files 空）
 * 展开显示口令全文+「复制口令」。
 *
 * 递活逻辑原样从 AssetDetailDialog（已删）上移：
 * - 「发到新会话」走 onCreateTask({ initialPrompt: buildAssetTryPrompt(asset) })。
 * - 「发到当前会话」写 requestComposerTextInsert（workspace 级单槽，草稿与真会话的
 *   focused composer 都能消费）并 onOpenChat 切回会话视图；无活动会话（hasActiveChat）
 *   或只读（readOnly）时不渲染。
 */
import { useEffect, useState } from "react";
import {
  CheckIcon,
  CodeXmlIcon,
  CopyIcon,
  MessageSquareTextIcon,
  SendIcon,
} from "lucide-react";
import {
  CodeBlock,
  CodeBlockHeader,
  CodeBlockLanguageSelector,
  CodeBlockLanguageSelectorContent,
  CodeBlockLanguageSelectorItem,
  CodeBlockLanguageSelectorTrigger,
  CodeBlockLanguageSelectorValue,
} from "@/components/ai-elements/code-block.js";
import { Button } from "@/components/ui/button.js";
import { toast } from "@/components/ui/toast.js";
import type { CreateTaskRequest } from "@/app-shell/types.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { useZCodeSessionStore } from "@/store/zcodeSessionStore.js";
import type { AssetFile, AssetManifest } from "./catalog/types.js";
import { buildAssetTryPrompt } from "./assetTryPrompt.js";
import { AssetPreviewFrame } from "./AssetPreviewFrame.js";
import { useInView } from "./useInView.js";

/** locale 展示名：En 字段缺失回退中文原字段（titleEn ?? title）。 */
export function resolveDisplayTitle(manifest: AssetManifest, locale: string): string {
  return locale === "en-US" ? (manifest.titleEn ?? manifest.title) : manifest.title;
}

/** 可复制的图纸正文：prompt 类货图纸为空，口令本身就是货。 */
function resolveBlueprintFiles(asset: AssetManifest): AssetFile[] {
  return asset.files.length > 0
    ? asset.files
    : [{ name: asset.title, language: "text", content: asset.prompt }];
}

/** 递活接线（S4，原 AssetDetailDialog 的 props 原样上移）：由 WorkspaceShellLayout 照 plugin-store 同链传下来。 */
export interface AssetCardActions {
  /** 发到当前会话的目标 workspace；identity 缺省时状态写 path 桶。 */
  workspacePath: string;
  workspaceIdentity?: string;
  /** 当前 workspace 是否有活动会话视图；无则不渲染「发到当前会话」。 */
  hasActiveChat: boolean;
  /** 活动 workspace 只读时隐藏两个递活按钮（与 onCreateTask 的只读守卫同待遇）。 */
  readOnly?: boolean;
  /** 递活到当前会话后切回会话视图；展厅是独立主视图，不切回去插入看不见反馈。 */
  onOpenChat?: () => void;
  onCreateTask?: (request?: CreateTaskRequest) => void;
}

export function AssetDemoCard({
  manifest,
  index,
  locale,
  workspacePath,
  workspaceIdentity,
  hasActiveChat,
  readOnly = false,
  onOpenChat,
  onCreateTask,
}: {
  manifest: AssetManifest;
  /** 展示序号（01 起，随过滤结果重排）。 */
  index: number;
  locale: string;
} & AssetCardActions) {
  const { intl } = useZCodeIntl();
  const { ref, inView } = useInView<HTMLDivElement>();
  const blueprintFiles = resolveBlueprintFiles(manifest);
  const isPromptAsset = manifest.files.length === 0;
  const [codeOpen, setCodeOpen] = useState(false);
  const [activeFileName, setActiveFileName] = useState(blueprintFiles[0]!.name);
  const [copied, setCopied] = useState(false);
  const activeFile = blueprintFiles.find((file) => file.name === activeFileName) ?? blueprintFiles[0]!;

  useEffect(() => {
    if (!copied) {
      return;
    }
    const timer = window.setTimeout(() => setCopied(false), 2000);
    return () => window.clearTimeout(timer);
  }, [copied]);

  // 递活消息三种动作共用同一组装（技术设计 §3）：只预填输入框，发送权在用户手里。
  const tryPrompt = buildAssetTryPrompt(manifest);
  const handleSendToNewChat = () => {
    onCreateTask?.({ initialPrompt: tryPrompt });
  };
  const handleSendToCurrentChat = () => {
    useZCodeSessionStore
      .getState()
      .requestComposerTextInsert(workspacePath, tryPrompt, workspaceIdentity);
    // 展厅是独立主视图：不切回会话，插入要等 composer 挂载才兑现，用户会以为点了没反应。
    onOpenChat?.();
  };

  // 复制当前展开文件内容（prompt 货即口令全文）；成功 toast，失败 toast 指引手动复制。
  const handleCopy = async () => {
    if (typeof navigator === "undefined" || !navigator.clipboard?.writeText) {
      toast(
        intl.formatMessage(
          { id: "assetLibrary.detail.copyFailed" },
          { error: "clipboard-unavailable" },
        ),
      );
      return;
    }
    try {
      await navigator.clipboard.writeText(activeFile.content);
      setCopied(true);
      toast(intl.formatMessage({ id: "assetLibrary.detail.copied" }));
    } catch (error) {
      toast(
        intl.formatMessage({ id: "assetLibrary.detail.copyFailed" }, {
          error: error instanceof Error ? error.message : String(error),
        }),
      );
    }
  };

  const title = resolveDisplayTitle(manifest, locale);
  const description =
    locale === "en-US" ? (manifest.descriptionEn ?? manifest.description) : manifest.description;
  const license = manifest.source?.license ?? intl.formatMessage({ id: "assetLibrary.license.selfMade" });
  const copyLabel = intl.formatMessage({
    id: isPromptAsset ? "assetLibrary.card.copyPrompt" : "assetLibrary.detail.copy",
  });
  const copyAriaLabel = intl.formatMessage({
    id: isPromptAsset ? "assetLibrary.card.copyPromptAria" : "assetLibrary.detail.copyAria",
  });
  const copyIcon = copied ? (
    <CheckIcon className="size-3.5" aria-hidden="true" />
  ) : (
    <CopyIcon className="size-3.5" aria-hidden="true" />
  );

  return (
    <article
      data-testid="asset-library-card"
      data-asset-id={manifest.id}
      className="flex flex-col gap-3 rounded-2xl border border-border bg-card p-4 transition-colors hover:border-border-hover"
    >
      <header className="flex flex-wrap items-start justify-between gap-x-3 gap-y-2">
        <div className="flex min-w-0 items-baseline gap-3">
          <span
            aria-hidden="true"
            className="font-mono text-ui-xl leading-none text-foreground-subtlest tabular-nums"
          >
            {String(index + 1).padStart(2, "0")}
          </span>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="text-ui-base font-semibold text-foreground">{title}</h2>
              <span className="rounded-full bg-surface-hover px-1.5 py-0.5 text-ui-sm leading-none text-foreground-subtle">
                {intl.formatMessage({ id: `assetLibrary.category.${manifest.category}` })}
              </span>
              <span
                className="rounded-full border border-border px-1.5 py-0.5 text-ui-sm leading-none text-foreground-subtle"
                aria-label={intl.formatMessage({ id: "assetLibrary.license.aria" }, { license })}
              >
                {license}
              </span>
            </div>
            <p className="mt-1 truncate text-ui-sm leading-snug text-foreground-subtle">{description}</p>
          </div>
        </div>
        <div
          role="group"
          aria-label={intl.formatMessage({ id: "assetLibrary.card.actionsLabel" })}
          className="flex shrink-0 items-center gap-1"
        >
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            data-testid="asset-library-code-toggle"
            aria-pressed={codeOpen}
            aria-label={intl.formatMessage({
              id: codeOpen ? "assetLibrary.card.collapseCodeAria" : "assetLibrary.card.codeAria",
            })}
            title={intl.formatMessage({
              id: codeOpen ? "assetLibrary.card.collapseCode" : "assetLibrary.card.code",
            })}
            onClick={() => setCodeOpen((open) => !open)}
          >
            <CodeXmlIcon className="size-3.5" aria-hidden="true" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            data-testid="asset-library-copy"
            aria-label={copyAriaLabel}
            title={copyLabel}
            onClick={() => {
              void handleCopy();
            }}
          >
            {copyIcon}
          </Button>
          {readOnly ? null : (
            <>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                data-testid="asset-library-send-new-chat"
                aria-label={intl.formatMessage({ id: "assetLibrary.detail.sendToNewChatAria" })}
                onClick={handleSendToNewChat}
              >
                <SendIcon className="size-3" aria-hidden="true" data-icon="inline-start" />
                {intl.formatMessage({ id: "assetLibrary.detail.sendToNewChat" })}
              </Button>
              {hasActiveChat ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  data-testid="asset-library-send-current-chat"
                  aria-label={intl.formatMessage({ id: "assetLibrary.detail.sendToCurrentChatAria" })}
                  onClick={handleSendToCurrentChat}
                >
                  <MessageSquareTextIcon className="size-3" aria-hidden="true" data-icon="inline-start" />
                  {intl.formatMessage({ id: "assetLibrary.detail.sendToCurrentChat" })}
                </Button>
              ) : null}
            </>
          )}
        </div>
      </header>

      {/* 演示区：进视口挂真沙箱、出视口卸载回落骨架；overflow-hidden 圆角边框钉住预览溢出。 */}
      <div
        ref={ref}
        data-testid="asset-library-preview"
        aria-label={intl.formatMessage({ id: "assetLibrary.detail.previewLabel" })}
        className="h-[380px] shrink-0 overflow-hidden rounded-xl border border-border bg-background"
      >
        {inView ? (
          <AssetPreviewFrame previewHtml={manifest.previewHtml} title={manifest.title} />
        ) : (
          <div aria-hidden="true" className="flex h-full w-full flex-col gap-2 p-3">
            <span className="motion-safe:animate-pulse h-1/2 rounded-md bg-surface" />
            <span className="motion-safe:animate-pulse h-5 w-3/4 rounded-md bg-surface" />
            <span className="motion-safe:animate-pulse h-5 w-1/2 rounded-md bg-surface" />
          </div>
        )}
      </div>

      {codeOpen ? (
        <div className="flex flex-col gap-2" data-testid="asset-library-code-panel">
          {isPromptAsset ? (
            <div className="rounded-xl border border-border bg-muted/40 p-3">
              <p
                className="whitespace-pre-wrap text-ui-sm leading-relaxed text-foreground"
                data-testid="asset-library-prompt-text"
              >
                {manifest.prompt}
              </p>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="mt-2"
                data-testid="asset-library-copy-prompt"
                aria-label={intl.formatMessage({ id: "assetLibrary.card.copyPromptAria" })}
                onClick={() => {
                  void handleCopy();
                }}
              >
                {copyIcon}
                {intl.formatMessage({ id: "assetLibrary.card.copyPrompt" })}
              </Button>
            </div>
          ) : (
            <>
              {blueprintFiles.length > 1 ? (
                <CodeBlockLanguageSelector value={activeFileName} onValueChange={setActiveFileName}>
                  <CodeBlockLanguageSelectorTrigger
                    className="self-start"
                    aria-label={intl.formatMessage({ id: "assetLibrary.card.fileSelect" })}
                  >
                    <CodeBlockLanguageSelectorValue />
                  </CodeBlockLanguageSelectorTrigger>
                  <CodeBlockLanguageSelectorContent>
                    {blueprintFiles.map((file) => (
                      <CodeBlockLanguageSelectorItem key={file.name} value={file.name}>
                        <span className="font-mono">{file.name}</span>
                      </CodeBlockLanguageSelectorItem>
                    ))}
                  </CodeBlockLanguageSelectorContent>
                </CodeBlockLanguageSelector>
              ) : null}
              <CodeBlock
                code={activeFile.content}
                language={activeFile.language}
                // 超长文件面板内滚动，不撑破卡片（技术设计 §6）
                contentClassName="max-h-72 overflow-auto"
                data-testid="asset-library-code"
              >
                <CodeBlockHeader displayFile={activeFile.name} />
              </CodeBlock>
            </>
          )}
        </div>
      ) : null}
    </article>
  );
}
