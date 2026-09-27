/**
 * 素材详情弹层（技术设计 §4）：真 iframe 预览 + 图纸 tab + 复制。
 *
 * iframe 现挂现卸——asset 为 null 即卸载整个 body，弹层一关沙箱就回收。
 * 「发到会话」按钮归 S4 递活片，本片不渲染。
 * Radix Dialog 自带焦点圈/Esc 关闭/焦点归还；DialogContent 的关闭按钮沿用组件默认。
 */
import { useEffect, useState } from "react";
import { Check, Copy } from "lucide-react";
import { Button } from "@/components/ui/button.js";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog.js";
import { toast } from "@/components/ui/toast.js";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { AssetPreviewFrame } from "./AssetPreviewFrame.js";
import type { AssetFile, AssetManifest } from "./catalog/types.js";

/** 可复制的图纸正文：prompt 类货图纸为空，口令本身就是货。 */
function resolveBlueprintFiles(asset: AssetManifest): AssetFile[] {
  return asset.files.length > 0
    ? asset.files
    : [{ name: asset.title, language: "text", content: asset.prompt }];
}

/** key={asset.id} 挂载：切货即重置 tab/复制态。 */
function AssetDetailBody({ asset }: { asset: AssetManifest }) {
  const { intl } = useZCodeIntl();
  const blueprintFiles = resolveBlueprintFiles(asset);
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

  return (
    <div className="flex min-h-0 flex-col gap-4">
      <DialogHeader>
        <DialogTitle data-testid="asset-library-detail-title">{asset.title}</DialogTitle>
        <DialogDescription>{asset.description}</DialogDescription>
      </DialogHeader>
      <div
        className="h-64 shrink-0 overflow-hidden rounded-xl border border-border bg-background"
        data-testid="asset-library-detail-preview"
        aria-label={intl.formatMessage({ id: "assetLibrary.detail.previewLabel" })}
      >
        <AssetPreviewFrame previewHtml={asset.previewHtml} title={asset.title} />
      </div>
      <div className="flex min-h-0 flex-col gap-2">
        <div className="flex items-center justify-between gap-2">
          <h3 className="text-ui-sm font-semibold text-foreground">
            {asset.files.length > 0
              ? intl.formatMessage({ id: "assetLibrary.detail.blueprint" })
              : intl.formatMessage({ id: "assetLibrary.detail.promptBlueprint" })}
            {asset.files.length > 1 ? ` · ${asset.files.length}` : ""}
          </h3>
          <div className="flex items-center gap-2">
            <span className="rounded-full bg-surface-hover px-1.5 py-0.5 text-ui-sm leading-none text-foreground-subtle">
              {activeFile.language}
            </span>
            <Button
              type="button"
              variant="secondary"
              size="sm"
              data-testid="asset-library-detail-copy"
              aria-label={intl.formatMessage({ id: "assetLibrary.detail.copyAria" })}
              onClick={() => {
                void handleCopy();
              }}
            >
              {copied ? (
                <Check className="size-3.5" aria-hidden="true" />
              ) : (
                <Copy className="size-3.5" aria-hidden="true" />
              )}
              {copied
                ? intl.formatMessage({ id: "assetLibrary.detail.copied" })
                : intl.formatMessage({ id: "assetLibrary.detail.copy" })}
            </Button>
          </div>
        </div>
        {blueprintFiles.length > 1 ? (
          <Tabs value={activeFile.name} onValueChange={setActiveFileName}>
            <TabsList className="max-w-full flex-wrap">
              {blueprintFiles.map((file) => (
                <TabsTrigger key={file.name} value={file.name}>
                  <span className="max-w-40 truncate font-mono">{file.name}</span>
                </TabsTrigger>
              ))}
            </TabsList>
            <TabsContent value={activeFile.name} className="mt-2">
              <pre
                tabIndex={0}
                data-testid="asset-library-detail-code"
                className="max-h-64 overflow-auto rounded-lg bg-muted p-3 font-mono text-ui-sm leading-relaxed text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-input-border-focused"
              >
                <code>{activeFile.content}</code>
              </pre>
            </TabsContent>
          </Tabs>
        ) : (
          <pre
            tabIndex={0}
            data-testid="asset-library-detail-code"
            className="max-h-64 overflow-auto rounded-lg bg-muted p-3 font-mono text-ui-sm leading-relaxed text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-input-border-focused"
          >
            <code>{activeFile.content}</code>
          </pre>
        )}
      </div>
    </div>
  );
}

/**
 * 单例弹层：asset 为 null 即关（并卸载 iframe）。key 保证切货时状态不串。
 * 注意 Tabs 的 value 归 body 内部管，这里只负责开关。
 */
export function AssetDetailDialog({
  asset,
  onClose,
}: {
  asset: AssetManifest | null;
  onClose: () => void;
}) {
  return (
    <Dialog
      open={asset !== null}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
    >
      <DialogContent className="max-w-2xl" data-testid="asset-library-detail-dialog">
        {asset ? <AssetDetailBody key={asset.id} asset={asset} /> : null}
      </DialogContent>
    </Dialog>
  );
}
