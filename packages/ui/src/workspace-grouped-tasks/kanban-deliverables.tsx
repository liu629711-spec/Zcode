// 派活台卡片第 5 行"成卡/交付行"：ZCodeTaskMeta.deliverables 是 unknown（tasks 旁路列是 json
// text，读侧坏 JSON 已解析为 undefined；无 schema——按 unknown JSON 防御渲染，本卡不立 schema）。
// 分类收口在 mapDispatchDeskDeliverables：可辨认文件项（path/file 非空字符串）成迷你卡并给预览入口；
// 其余任意 JSON 一行摘要就地截断（本地 clip 纪律，不 import shared）。
import { EyeIcon, FileTextIcon } from "lucide-react";
import type { FileCodeViewerSource } from "@/lib/codeViewer.js";
import { getPathLeaf } from "@/lib/path.js";
import { Button } from "@/components/ui/button.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import {
  TID_DISPATCH_DESK_DELIVERABLE,
  TID_DISPATCH_DESK_DELIVERABLE_OPEN,
} from "@zcode/shared";

export type DispatchDeskDeliverableRow =
  | { kind: "file"; title: string; subtitle: string | null }
  | { kind: "summary"; text: string };

/** 摘要行就地截断封顶：任何 JSON 产物的 stringify 都不把卡片 DOM 撑爆。 */
const SUMMARY_MAX_CHARS = 200;

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** 辨认一个文件项：path 或 file 字段是非空非纯空白字符串；近似字段名/非字符串一律不认，降级摘要。 */
function recognizeFileItem(value: unknown): { title: string; subtitle: string | null } | null {
  if (!isPlainRecord(value)) return null;
  const path = [value.path, value.file]
    .find((candidate): candidate is string => typeof candidate === "string" && candidate.trim().length > 0)
    ?.trim();
  if (!path) return null;
  // 副标题 = 除 path/file 外的其余可辨标量字段（对象/数组不进摘要行）。
  const isScalar = (val: unknown): val is string | number | boolean =>
    typeof val === "string" || typeof val === "number" || typeof val === "boolean";
  const subtitle = Object.entries(value)
    .filter(([key, val]) => key !== "path" && key !== "file" && isScalar(val))
    .map(([key, val]) => `${key}: ${String(val)}`)
    .join(" · ");
  return { title: path, subtitle: subtitle.length > 0 ? subtitle : null };
}

/** deliverables → 成卡行：undefined 缺席；文件项对象（或纯文件项数组）成卡；其余任意 JSON 一行摘要。 */
export function mapDispatchDeskDeliverables(
  deliverables: unknown,
): DispatchDeskDeliverableRow[] | null {
  if (deliverables === undefined) return null;
  const single = recognizeFileItem(deliverables);
  if (single) return [{ kind: "file", ...single }];
  if (Array.isArray(deliverables) && deliverables.length > 0) {
    const items = deliverables.map(recognizeFileItem);
    if (items.every((item): item is NonNullable<typeof item> => item !== null)) {
      return items.map((item) => ({ kind: "file" as const, ...item }));
    }
  }
  const text = JSON.stringify(deliverables) ?? "";
  return [
    {
      kind: "summary",
      text: text.length <= SUMMARY_MAX_CHARS ? text : text.slice(0, SUMMARY_MAX_CHARS),
    },
  ];
}

/** 迷你成卡行：32px 图标方块 + 标题/副标题 truncate + 右侧预览入口（仅可辨认 path 的项）。 */
export function DispatchDeskCardDeliverables(props: {
  deliverables: unknown;
  onOpenCodeViewer?: (source: FileCodeViewerSource) => void;
}) {
  const { intl } = useZCodeIntl();
  const { deliverables, onOpenCodeViewer } = props;
  const rows = mapDispatchDeskDeliverables(deliverables);
  if (!rows) return null;
  return (
    <div className="mt-1.5 flex flex-col gap-1">
      {rows.map((row, index) =>
        row.kind === "file" ? (
          <div
            key={`${index}:${row.title}`}
            data-testid={TID_DISPATCH_DESK_DELIVERABLE}
            className="flex min-w-0 items-center gap-2 rounded-lg border border-card-border bg-card p-2"
          >
            <span
              aria-hidden="true"
              className="flex size-8 shrink-0 items-center justify-center rounded-md bg-background"
            >
              <FileTextIcon className="size-4 text-foreground-subtle" />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-ui-sm font-medium text-foreground" title={row.title}>
                {row.title}
              </span>
              {row.subtitle ? (
                <span
                  className="block truncate text-ui-sm text-foreground-subtlest"
                  title={row.subtitle}
                >
                  {row.subtitle}
                </span>
              ) : null}
            </span>
            {onOpenCodeViewer ? (
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label={intl.formatMessage({ id: "dispatchDesk.deliverable.open" })}
                data-testid={TID_DISPATCH_DESK_DELIVERABLE_OPEN}
                // 卡根是拖拽 handle：按钮按下必须断开 sensor 事件链（同卡上动作按钮）。
                onPointerDown={(event) => event.stopPropagation()}
                onMouseDown={(event) => event.stopPropagation()}
                onClick={(event) => {
                  event.stopPropagation();
                  onOpenCodeViewer({
                    type: "file",
                    title: getPathLeaf(row.title) || row.title,
                    path: row.title,
                  });
                }}
              >
                <EyeIcon aria-hidden="true" />
              </Button>
            ) : null}
          </div>
        ) : (
          <div
            key={`summary:${index}`}
            className="truncate text-ui-sm text-foreground-subtlest"
            title={row.text}
          >
            {row.text}
          </div>
        ),
      )}
    </div>
  );
}
