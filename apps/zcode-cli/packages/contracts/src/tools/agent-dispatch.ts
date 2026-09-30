// ============================================================
// AgentDispatch tool - hand a task to another agent's resident session (D29/D1)
// ============================================================

import { z } from "zod";
import { toToolJsonSchema } from "./json-schema.js";

const nonEmptyString = z.string().trim().min(1);

export const AgentDispatchInputSchema = z
  .object({
    agent: nonEmptyString
      .optional()
      .describe(
        "Target agent of this workspace: pass the agent's agentId (the uuid from its agent file) when known, otherwise its exact name. Ambiguous names are rejected, never guessed. Omit ONLY when newSession=true to spawn an unbadged ordinary session (no agent identity, memory or badge) with the task as its first work order.",
      ),
    task: nonEmptyString.describe(
      "Complete task description the target agent runs in its own session. Include all instructions and context needed to do the work without relying on this conversation.",
    ),
    newSession: z
      .boolean()
      .optional()
      .describe(
        "true starts a fresh session: a new persona session for the named agent, or - when no agent is given - an unbadged ordinary session. Default (false) delivers the task into the agent's latest persona session, creating one if none exists.",
      ),
    title: z
      .string()
      .optional()
      .describe(
        "Short human title for the conversation when a new session is created (e.g. 发个帅哥问候, written in the user's language). Shown in the session list. Ignored when delivering into an existing session.",
      ),
    model: z
      .string()
      .optional()
      .describe(
        "Model this task runs on, e.g. 'deepseek-v4.1'. Omit to use the agent's own model. Unavailable or ambiguous names are rejected with the available options listed. In a spawned ordinary session (no agent) the model becomes that session's resident model from birth.",
      ),
    // 批次派单（工地卡）：batch_title 开批，batch_id 把后续工单并进同一批。
    // 模型面用 snake_case（工单批次是模型的新概念，名字与说明书逐字对齐，别混 camelCase）。
    batch_title: z
      .string()
      .trim()
      .min(1)
      .max(80)
      .optional()
      .describe(
        "Short job-site title for a BATCH of work orders (e.g. 登录页改造, in the user's language). Pass the SAME batch_title on every dispatch of one big task - they run in parallel and the conversation shows them as one aggregated job-site card. Omit for a standalone task.",
      ),
    batch_id: z
      .string()
      .trim()
      .min(1)
      .max(128)
      .optional()
      .describe(
        "Batch identity: pass back the batch_id from a previous dispatch result to add more work orders to the same job site later (e.g. after reading a receipt). Only honored together with batch_title; omit to start a new batch.",
      ),
  })
  .strict()
  // 未点名（对齐稿 §三 #2/#5）：不点名员工只允许派生新普通会话；否则无落点。
  .refine((input) => input.agent !== undefined || input.newSession === true, {
    message: "agent is required unless newSession is true",
    path: ["agent"],
  });
export type AgentDispatchInput = z.infer<typeof AgentDispatchInputSchema>;
export const AgentDispatchInputJsonSchema = toToolJsonSchema(AgentDispatchInputSchema);

export const AgentDispatchOutputSchema = z
  .object({
    targetSessionId: nonEmptyString,
    // 无 persona 普通会话（agent 缺省派生）没有档案名：缺席而非空串（additive 放宽）。
    agentName: nonEmptyString.optional(),
    agentId: nonEmptyString.optional(),
    delivery: z.enum(["started", "queued"]),
    createdSession: z.boolean(),
    model: nonEmptyString.optional(),
    // 批次回显（带 batch_title 派单时必在）：模型回传 batch_id 即可往同一工地续派。
    workOrderId: nonEmptyString.optional(),
    batchId: nonEmptyString.optional(),
    batchTitle: nonEmptyString.optional(),
    message: nonEmptyString,
  })
  .strict();
export type AgentDispatchOutput = z.infer<typeof AgentDispatchOutputSchema>;
export const AgentDispatchOutputJsonSchema = toToolJsonSchema(AgentDispatchOutputSchema);
