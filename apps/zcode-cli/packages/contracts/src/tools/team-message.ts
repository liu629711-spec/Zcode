// ============================================================
// TeamMessage tool - 队内直达消息（团队看板批3 2026-10-05）
// ============================================================
// 同事串工位：只发同队、防冒名（from 系统铸造）、必留痕（队长台账 + 看板
// 通讯区）。与 AgentDispatch 分工：消息是一句话的唤醒（不建工单台账、不接
// 回执线、不计频控在飞），派活仍走 AgentDispatch。

import { z } from "zod";
import { toToolJsonSchema } from "./json-schema.js";

const nonEmptyString = z.string().trim().min(1);

export const TeamMessageInputSchema = z
  .object({
    to: nonEmptyString
      .describe(
        "Teammate to message: the agent's agentId when known, otherwise its exact name. Must be a member of YOUR current team (dispatched into the same batch); messaging across teams is rejected.",
      ),
    message: z
      .string()
      .trim()
      .min(1)
      .max(4000)
      .describe(
        "Message body in plain text (e.g. 'login 接口的字段是不是改了？给我对一下'). It wakes the teammate's session as a short message turn - not a work order, so no receipt comes back; for handing over actual work use AgentDispatch instead.",
      ),
  })
  .strict();
export type TeamMessageInput = z.infer<typeof TeamMessageInputSchema>;
export const TeamMessageInputJsonSchema = toToolJsonSchema(TeamMessageInputSchema);

export const TeamMessageOutputSchema = z
  .object({
    targetSessionId: nonEmptyString,
    toName: nonEmptyString,
    batchId: nonEmptyString.optional(),
    delivery: z.literal("sent"),
    message: nonEmptyString,
  })
  .strict();
export type TeamMessageOutput = z.infer<typeof TeamMessageOutputSchema>;
export const TeamMessageOutputJsonSchema = toToolJsonSchema(TeamMessageOutputSchema);
