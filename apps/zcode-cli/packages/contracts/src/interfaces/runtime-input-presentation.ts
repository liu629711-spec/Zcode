import { z } from "zod";

/** 来源和实际消费形态的单一标记；缺失的旧历史不追溯转换。 */
export const RuntimeInputPresentationSchema = z.enum([
  "user_steer",
  "coordinator_steer",
  "coordinator_input",
  "subagent_reply_steer",
  "subagent_reply",
  "task_notification_steer",
  "task_notification",
  // 派单工单唤醒轮（D29）：目标会话里工单 carrier 的落库形态标记。
  "agent_work_order",
  // 派单回执轮（D29/D3）：发起方会话里回执 carrier 的落库形态标记。
  "agent_work_order_receipt",
  // 批次质检轮（纪律协议批）：批次全部收口后在发起方会话自动开的验货轮。
  "agent_work_order_batch_qc",
  // 工单复盘轮（学习沉淀 v1，Hermes background_review 同款）：员工交活后在自己
  // 会话自动开的经验沉淀轮——产出写进技能册，不产生回执。
  "agent_work_order_debrief",
]);
export type RuntimeInputPresentation = z.infer<typeof RuntimeInputPresentationSchema>;

export function parseRuntimeInputPresentation(
  value: unknown,
): RuntimeInputPresentation | undefined {
  const result = RuntimeInputPresentationSchema.safeParse(value);
  return result.success ? result.data : undefined;
}
