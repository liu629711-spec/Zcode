// ============================================================
// AgentDispatch tool - hand a task to another agent's resident session (D29/D1)
// ============================================================

import { z } from "zod";
import { toToolJsonSchema } from "./json-schema.js";

const nonEmptyString = z.string().trim().min(1);

export const AgentDispatchInputSchema = z
  .object({
    agent: nonEmptyString
      .describe(
        "Target agent of this workspace: pass the agent's agentId (the uuid from its agent file) when known, otherwise its exact name. Ambiguous names are rejected, never guessed.",
      ),
    task: nonEmptyString.describe(
      "Complete task description the target agent runs in its own session. Include all instructions and context needed to do the work without relying on this conversation.",
    ),
    newSession: z
      .boolean()
      .optional()
      .describe(
        "true starts a fresh persona session for the agent. Default (false) delivers the task into the agent's latest persona session, creating one if none exists.",
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
        "Model this task runs on, e.g. 'deepseek-v4.1'. Omit to use the agent's own model. Unavailable or ambiguous names are rejected with the available options listed. Applies to this task only.",
      ),
  })
  .strict();
export type AgentDispatchInput = z.infer<typeof AgentDispatchInputSchema>;
export const AgentDispatchInputJsonSchema = toToolJsonSchema(AgentDispatchInputSchema);

export const AgentDispatchOutputSchema = z
  .object({
    targetSessionId: nonEmptyString,
    agentName: nonEmptyString,
    agentId: nonEmptyString.optional(),
    delivery: z.enum(["started", "queued"]),
    createdSession: z.boolean(),
    model: nonEmptyString.optional(),
    message: nonEmptyString,
  })
  .strict();
export type AgentDispatchOutput = z.infer<typeof AgentDispatchOutputSchema>;
export const AgentDispatchOutputJsonSchema = toToolJsonSchema(AgentDispatchOutputSchema);
