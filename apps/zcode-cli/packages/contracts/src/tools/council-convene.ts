// ============================================================
// CouncilConvene tool - convene a roundtable council (真会议)
// ============================================================
// 圆桌会（2026-10-03）：主智能体提议、用户批准后，N 个员工智能体作为对立面
// 攻角席位在各自会话独立发言、交叉质询、行尾交裁定行，代码算票出决议。
// 工具面只盖「提议开会」这一次批准（会议是一次批准而非 N 次派单批准）；
// 座次、轮次推进、收票、重询全是代码驱动（core 编排），模型不传 round/lens。
// 会议编排参数（councilId/round/seat*）只存在于派单端口与信封（AgentDispatch
// Request 的 council* 五参），模型面不暴露——模型逐席传 lens/round 必错。

import { z } from "zod";
import { toToolJsonSchema } from "./json-schema.js";
import { COUNCIL_SEAT_LENS_IDS } from "../interfaces/council.js";

export const COUNCIL_CONVENE_TOOL_NAME = "CouncilConvene";

const nonEmptyString = z.string().trim().min(1);

export const CouncilConveneInputSchema = z
  .object({
    topic: nonEmptyString
      .max(4000)
      .describe(
        "The motion under review, in the user's language: for a plan meeting the proposal/direction to review BEFORE work starts; for an acceptance meeting the completed work to accept. Pass the user's words (or a faithful summary) - the seats deliberate on exactly this text.",
      ),
    kind: z
      .enum(["plan", "acceptance"])
      .describe(
        "Meeting kind: plan = pre-work proposal review (decides direction/content before anyone builds); acceptance = post-completion acceptance of finished work.",
      ),
    seats: z
      .array(nonEmptyString)
      .min(2)
      .max(8)
      .describe(
        "Agent names (or agentIds) to seat, in order - seat order is fixed by the system and each seat gets an opposing review angle automatically. Never seat the agent who did the work being reviewed. 2-4 seats is the sweet spot; every extra seat multiplies token cost.",
      ),
    materials: z
      .string()
      .max(8192)
      .optional()
      .describe(
        "Optional supporting material attached to every seat's envelope (file paths, requirement quotes, diff summary). Keep it bounded; the seats can read the workspace themselves.",
      ),
    title: z
      .string()
      .trim()
      .min(1)
      .max(80)
      .optional()
      .describe(
        "Short human title for the council card (e.g. 登录页方案评审, in the user's language). Omit to derive it from the topic.",
      ),
  })
  .strict();
export type CouncilConveneInput = z.infer<typeof CouncilConveneInputSchema>;
export const CouncilConveneInputJsonSchema = toToolJsonSchema(CouncilConveneInputSchema);

export const CouncilConveneOutputSchema = z
  .object({
    councilId: nonEmptyString,
    kind: z.enum(["plan", "acceptance"]),
    topic: nonEmptyString,
    round: z.union([z.literal(1), z.literal(2)]),
    status: z.enum(["proposed", "running", "approved", "rejected", "deadlocked", "cancelled"]),
    seats: z
      .array(
        z
          .object({
            index: z.number().int().nonnegative(),
            agentName: nonEmptyString,
            lens: z.enum(COUNCIL_SEAT_LENS_IDS),
          })
          .strict(),
      )
      .min(1),
    /** 已成功派单的席位数；派单失败的席位按缺席弃权处理并在 failedSeats 如实列出。 */
    dispatchedSeats: z.number().int().nonnegative(),
    failedSeats: z
      .array(
        z
          .object({ agentName: nonEmptyString, reason: nonEmptyString })
          .strict(),
      )
      .optional(),
    message: nonEmptyString,
  })
  .strict();
export type CouncilConveneOutput = z.infer<typeof CouncilConveneOutputSchema>;
export const CouncilConveneOutputJsonSchema = toToolJsonSchema(CouncilConveneOutputSchema);
