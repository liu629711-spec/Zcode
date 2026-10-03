// ============================================================
// council/evidence 的证据抽取纯规则（2026-10-03 圆桌会独立页刀5）：
// 从召集方会话的持久消息里找出轮头 originMeta.councilId 匹配的唤醒轮
// （席位回执轮 + 主席合议轮），把每个唤醒轮连同其后的 assistant 发言
// 组装成与 UI CouncilEvidenceUnit（packages/ui/src/v4/councilMeeting.ts）
// 结构对齐的证据单元——UI 据此 selectCouncilMeetings 派生会议模型。
//
// 轮语义与冷恢复（transcript-hydration）同一口径：唤醒 carrier（user 角色、
// providerContextOnly）开轮，其后 assistant 消息是本轮发言，直到下一条
// user 消息（真实用户输入或下一个 carrier）。providerContextOnly 的
// assistant（副屏继承历史）不算发言。
//
// 窄输入面：不 import contracts/shared，MessageWithParts 由调用方投影成
// 本形状（同 councilMeeting.ts 的 CouncilEvidenceUnit 纪律，纯函数可独立单测）。
// ============================================================

/** 消息里一个 part 的窄形状（MessagePart 结构兼容）。 */
export interface CouncilEvidenceTextPart {
  readonly type?: string;
  readonly text?: string;
  readonly ignored?: boolean;
}

/** 一条持久消息的窄形状（MessageWithParts 结构兼容，originMeta 已由调用方解出）。 */
export interface CouncilEvidenceSourceMessage {
  readonly id: string;
  readonly role: string;
  /** 调用方从 info.metadata.originMeta / 首个 text part metadata.originMeta 解出的原始值。 */
  readonly originMeta?: unknown;
  readonly parts?: readonly CouncilEvidenceTextPart[];
  /** 消息投影策略为 providerContextOnly（不进时间线的继承/合成消息）。 */
  readonly providerContextOnly?: boolean;
}

/** 与 UI CouncilEvidenceUnit 结构对齐的输出单元（originMeta 原样上抛，协议层再校验）。 */
export interface CouncilEvidenceUnitDraft {
  readonly key: string;
  readonly header: {
    readonly origin: "backgroundResult";
    readonly originMeta: Record<string, unknown>;
  };
  latestAssistantTextRow?: { text: string };
  assistantTextRows?: { text: string }[];
}

function originMetaRecordOf(value: unknown): Record<string, unknown> | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function isCouncilCarrier(message: CouncilEvidenceSourceMessage, councilId: string): boolean {
  if (message.role !== "user") return false;
  const originMeta = originMetaRecordOf(message.originMeta);
  return originMeta?.councilId === councilId;
}

function speechTextOf(parts: readonly CouncilEvidenceTextPart[] | undefined): string {
  return (parts ?? [])
    .filter(
      (part) =>
        // 宽松认 text part：持久 part 都带 type；缺 type 的按 text 兜底（同 textOfMessage）。
        (part.type === undefined || part.type === "text") && part.ignored !== true,
    )
    .map((part) => part.text ?? "")
    .join("");
}

/**
 * 抽取某场会议的全部证据单元（按消息顺序；畸形 carrier 不猜不抛）。
 * 只有 councilId 匹配的唤醒轮成单元；其余消息只承担轮边界职责。
 */
export function extractCouncilEvidenceUnits(
  messages: readonly CouncilEvidenceSourceMessage[],
  councilId: string,
): CouncilEvidenceUnitDraft[] {
  const units: CouncilEvidenceUnitDraft[] = [];
  let open: {
    unit: CouncilEvidenceUnitDraft;
    rows: { text: string }[];
  } | null = null;

  const close = () => {
    if (!open) return;
    if (open.rows.length > 0) {
      open.unit.assistantTextRows = open.rows;
      const latest = [...open.rows].reverse().find((row) => row.text.trim() !== "");
      if (latest) open.unit.latestAssistantTextRow = { text: latest.text };
    }
    units.push(open.unit);
    open = null;
  };

  for (const message of messages) {
    if (isCouncilCarrier(message, councilId)) {
      close();
      const originMeta = originMetaRecordOf(message.originMeta);
      if (!originMeta) continue;
      open = {
        unit: {
          key: message.id,
          header: { origin: "backgroundResult", originMeta },
        },
        rows: [],
      };
      continue;
    }
    if (!open) continue;
    if (message.role === "user") {
      // 任何 user 消息都开新轮（真实输入 / 其他 carrier）：本唤醒轮到此收口。
      close();
      continue;
    }
    if (message.role !== "assistant") continue;
    if (message.providerContextOnly) continue;
    const text = speechTextOf(message.parts);
    if (text.trim()) open.rows.push({ text });
  }
  close();
  return units;
}
