// ============================================================
// TeamPlan tool - 排班草案（团队看板批3b，2026-10-05 老板拍板 P1~P4）
// ============================================================
// 两阶段协议的上半场：用户要"先看怎么分工再开工"→ 队长用本工具排一张草案
// （谁干哪单、谁等谁），落台账 status=staged——不派单、不开工、不建工位。
// 用户在看板上改（teamPlanUpdate）、确认开工（teamPlanApprove，逐单走既有
// 派单端口，依赖持派自动生效）或放弃（teamPlanDiscard，二次确认）。

import { z } from "zod";
import { toToolJsonSchema } from "./json-schema.js";

const nonEmptyString = z.string().trim().min(1);

const planTaskSchema = z.object({
  task_key: nonEmptyString
    .max(128)
    .describe("Symbolic key of this task inside the plan (e.g. 'implementation'); downstream tasks reference it in depends_on."),
  task: nonEmptyString
    .max(8192)
    .describe("Complete standalone task description for the assignee (same discipline as AgentDispatch's task)."),
  assignee: nonEmptyString
    .describe("The agent's agentId when known, otherwise its exact name, from this workspace's roster. Every task must name its doer - unnamed work does not go into plans."),
  depends_on: z
    .array(nonEmptyString.max(128))
    .max(16)
    .optional()
    .describe("task_keys of other tasks in this plan that must complete first. Omit when there are no prerequisites."),
});

export const TeamPlanInputSchema = z
  .object({
    title: nonEmptyString
      .max(80)
      .describe("Short human title for the plan (becomes the batch_title on approval), in the user's language."),
    tasks: z
      .array(planTaskSchema)
      .min(1)
      .max(16)
      .describe("The task breakdown: 1-16 tasks, each with a symbolic key, a named doer from the roster, and optional dependencies."),
  })
  .strict()
  .refine(
    (input) => {
      const keys = new Set(input.tasks.map((task) => task.task_key));
      return input.tasks.every((task) => (task.depends_on ?? []).every((key) => keys.has(key)));
    },
    { message: "depends_on must reference task_keys defined in this plan", path: ["tasks"] },
  );
export type TeamPlanInput = z.infer<typeof TeamPlanInputSchema>;
export const TeamPlanInputJsonSchema = toToolJsonSchema(TeamPlanInputSchema);

export const TeamPlanOutputSchema = z
  .object({
    planId: nonEmptyString,
    taskCount: z.number().int().positive(),
    message: nonEmptyString,
  })
  .strict();
export type TeamPlanOutput = z.infer<typeof TeamPlanOutputSchema>;
export const TeamPlanOutputJsonSchema = toToolJsonSchema(TeamPlanOutputSchema);
