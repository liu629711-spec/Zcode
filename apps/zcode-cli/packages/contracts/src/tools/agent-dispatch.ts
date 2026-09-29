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
    message: nonEmptyString,
  })
  .strict();
export type AgentDispatchOutput = z.infer<typeof AgentDispatchOutputSchema>;
export const AgentDispatchOutputJsonSchema = toToolJsonSchema(AgentDispatchOutputSchema);
