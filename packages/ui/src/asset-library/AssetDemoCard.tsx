/**
 * 素材大演示卡（技术设计 §10 V2-1，beautifului.dev 式画廊的单件载体）。
 *
 * 序号+分类徽标+标题+一句话+右上角常显悬浮操作组（代码/复制/递活，ghost hover 提亮）；
 * 演示区 h-[240px] 内嵌真 AssetPreviewFrame。
 *
 * iframe 挂载策略（V3-1 真机反馈）：首次进视口（useInView，±150px）才挂载，
 * **挂载后永久保留 DOM**——出视口只把容器 display:none，绝不卸载。真机上反复
 * 挂卸 192KB iframe 是滚动抖动的根源；挂一次不卸后滚动零重挂载开销，50 件
 * 同挂的内存代价可控（字符串级 srcDoc）。骨架态只存在于首次进视口前。
 * 点 `</>` 在卡片下方内联展开代码面板（ai-elements CodeBlock/shiki 渲染+自带
 * CodeBlockCopyButton；多文件用其自带 selector 切换）；prompt 类货（files 空）
 * 展开显示口令全文+「复制口令」。
 *
 * 递活逻辑（技术设计 §10 V2-2 引用化）：
 * - 普通货先经 platform 把图纸静默落盘到 `<workspace>/.zcode/asset-library/<id>/`，
 *   消息 = 口令 + 逐文件引用链接（buildAssetReferenceMessage），智能体自己读文件；
 *   落盘失败（异常/不支持）toast 提示并退回 buildAssetTryPrompt 全量文本（原行为）。
 * - prompt 类货（files 空）不落盘，仍走 buildAssetTryPrompt（本就是口令语义）。
 * - 「发到新会话」走 onCreateTask({ initialPrompt })；「发到当前会话」写
 *   requestComposerTextInsert（workspace 级单槽）并 onOpenChat 切回会话视图；
 *   无活动会话（hasActiveChat）或只读（readOnly）时不渲染。递活期间按钮 disabled+转圈防双击。
 */
import { useEffect, useState } from "react";
import {
  CheckIcon,
  CodeXmlIcon,
  CopyIcon,
  LoaderIcon,
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
import { usePlatform } from "@/hooks/usePlatform.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { useZCodeSessionStore } from "@/store/zcodeSessionStore.js";
import type { ComposerMentionPrefill } from "@/store/zcodeSessionStoreTypes.js";
import type { AssetFile, AssetManifest } from "./catalog/types.js";
import { buildAssetTryPrompt } from "./assetTryPrompt.js";
import { buildAssetReferenceChipMessage } from "./assetReferenceMessage.js";
import {
  AssetPreviewFrame,
  PREVIEW_DESIGN_HEIGHT,
  PREVIEW_DESIGN_WIDTH,
} from "./AssetPreviewFrame.js";
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
  const platform = usePlatform();
  const { ref, inView } = useInView<HTMLDivElement>();
  // 首挂后不卸（V3-1）：inView 只管「第一次」挂载和之后的显示/隐藏。
  const [hasBeenInView, setHasBeenInView] = useState(false);
  useEffect(() => {
    if (inView) setHasBeenInView(true);
  }, [inView]);
  const blueprintFiles = resolveBlueprintFiles(manifest);
  const isPromptAsset = manifest.files.length === 0;
  const [codeOpen, setCodeOpen] = useState(false);
  const [isSending, setIsSending] = useState(false);
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

  // 递活消息组装（技术设计 §10 V2-2 + V3-2 chip 化）：普通货先落盘，消息 = 引用 chip +
  // 口令 + 落盘指引（不再铺代码）；失败兜底全量文本。两个按钮共用 deliver（预填不发送）。
  const tryPrompt = buildAssetTryPrompt(manifest);
  const sendToChat = async (
    deliver: (message: string, mention?: ComposerMentionPrefill) => void,
  ) => {
    if (isSending) return;
    setIsSending(true);
    let message = tryPrompt;
    let mention: ComposerMentionPrefill | undefined;
    if (!isPromptAsset) {
      try {
        if (!platform.assetLibraryWriteFiles) {
          throw new Error("asset_library_write_not_supported");
        }
        const { writtenPaths } = await platform.assetLibraryWriteFiles({
          workspacePath,
          relativeDir: `.zcode/asset-library/${manifest.id}`,
          files: manifest.files.map((file) => ({ name: file.name, content: file.content })),
        });
        const chipMessage = buildAssetReferenceChipMessage(manifest, writtenPaths, locale);
        message = chipMessage.text;
        mention = chipMessage.mention;
      } catch {
        toast(intl.formatMessage({ id: "assetLibrary.detail.writeFailedFallback" }));
      }
    }
    deliver(message, mention);
    setIsSending(false);
  };
  const handleSendToNewChat = () => {
    void sendToChat((message, mention) => {
      // chip 需要 initialPromptMention 才在编辑器里呈现为结构化节点（插件商店同范式）。
      onCreateTask?.({ initialPrompt: message, ...(mention ? { initialPromptMention: mention } : {}) });
    });
  };
  const handleSendToCurrentChat = () => {
    void sendToChat((message, mention) => {
      // 先切回会话视图：composer 挂载后才消费单槽（展厅是独立主视图，不切回等于无人消费）。
      onOpenChat?.();
      // 预填写进 workspace 单槽；mention 结构让 composer 渲染成 chip 而不是代码/路径原文。
      useZCodeSessionStore
        .getState()
        .requestComposerTextInsert(workspacePath, message, workspaceIdentity, mention);
    });
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
      {/* V3 重设计：头部两行固定高度——第一行 序号+标题+徽标+图标操作组（不换行），
          第二行描述单独 truncate。操作全改图标（带 title），文字按钮在窄卡上挤换行
          是旧版"错位/参差"的元凶之一。 */}
      <header className="flex flex-col gap-1.5">
        <div className="flex min-w-0 items-center gap-2">
          <span
            aria-hidden="true"
            className="font-mono text-ui-sm leading-none text-foreground-subtlest tabular-nums"
          >
            {String(index + 1).padStart(2, "0")}
          </span>
          <h2 className="min-w-0 truncate text-ui-base font-semibold text-foreground">{title}</h2>
          <span className="shrink-0 rounded-full bg-surface-hover px-1.5 py-0.5 text-ui-sm leading-none text-foreground-subtle">
            {intl.formatMessage({ id: `assetLibrary.category.${manifest.category}` })}
          </span>
          <span
            className="shrink-0 rounded-full border border-border px-1.5 py-0.5 text-ui-sm leading-none text-foreground-subtle"
            aria-label={intl.formatMessage({ id: "assetLibrary.license.aria" }, { license })}
          >
            {license}
          </span>
          <div
            role="group"
            aria-label={intl.formatMessage({ id: "assetLibrary.card.actionsLabel" })}
            className="ml-auto flex shrink-0 items-center gap-0.5"
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
                  size="icon-sm"
                  data-testid="asset-library-send-new-chat"
                  aria-label={intl.formatMessage({ id: "assetLibrary.detail.sendToNewChatAria" })}
                  title={intl.formatMessage({ id: "assetLibrary.detail.sendToNewChat" })}
                  disabled={isSending}
                  aria-busy={isSending}
                  onClick={handleSendToNewChat}
                >
                  {isSending ? (
                    <LoaderIcon className="size-3.5 animate-spin" aria-hidden="true" />
                  ) : (
                    <SendIcon className="size-3.5" aria-hidden="true" />
                  )}
                </Button>
                {hasActiveChat ? (
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    data-testid="asset-library-send-current-chat"
                    aria-label={intl.formatMessage({ id: "assetLibrary.detail.sendToCurrentChatAria" })}
                    title={intl.formatMessage({ id: "assetLibrary.detail.sendToCurrentChat" })}
                    disabled={isSending}
                    aria-busy={isSending}
                    onClick={handleSendToCurrentChat}
                  >
                    {isSending ? (
                      <LoaderIcon className="size-3.5 animate-spin" aria-hidden="true" />
                    ) : (
                      <MessageSquareTextIcon className="size-3.5" aria-hidden="true" />
                    )}
                  </Button>
                ) : null}
              </>
            )}
          </div>
        </div>
        <p className="truncate text-ui-sm leading-snug text-foreground-subtle">{description}</p>
      </header>

      {/* 演示舞台（V3 缩放舞台）：纵横比恒等于设计视口（880×550），AssetPreviewFrame 把
          demo 按设计视口渲染再等比缩进来——内容永远完整，不裁切、不错位、无滚动条。
          浅色 demo（三站原版多为浅色主题）在这层画框里是"作品照"，与深色 UI 的反差是
          有意的装裱，不再显得突兀。首次进视口挂真沙箱，之后永久保留 DOM（滚动零重挂载）。 */}
      <div
        ref={ref}
        data-testid="asset-library-preview"
        aria-label={intl.formatMessage({ id: "assetLibrary.detail.previewLabel" })}
        className="w-full shrink-0 overflow-hidden rounded-xl border border-border bg-background"
        style={{ aspectRatio: `${PREVIEW_DESIGN_WIDTH} / ${PREVIEW_DESIGN_HEIGHT}` }}
      >
        {hasBeenInView ? (
          <div className="h-full w-full" style={{ display: inView ? undefined : "none" }}>
            <AssetPreviewFrame previewHtml={manifest.previewHtml} title={manifest.title} />
          </div>
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
