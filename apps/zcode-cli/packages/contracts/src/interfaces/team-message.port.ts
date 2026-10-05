// ============================================================
// Team Message Port - 队内直达消息（团队看板批3 2026-10-05）
// ============================================================
// 同事串工位：成员 A 直接给同队的成员 B 发一句话，B 被唤醒回一句，不用事事
// 交活回老板传话。纪律（参照件 dsh-agent-teams 同款 + 我们自己的）：
//  - 防冒名：from 由执行器按发起会话的 persona 铸造，模型不可伪造；
//  - 只发队内：收发双方必须在同一批次（台账派单行对账）；
//  - 必留痕：每条消息在队长会话台账落一行，看板「通讯」区老板随时翻。

export interface TeamMessageRequest {  /** 收件人：工号优先，其次精确名（与派单同一解析纪律，歧义拒绝）。 */
  to: string;
  /** 消息正文（纯文本；投到对方工位作为一次消息轮，不产生回执）。 */
  message: string;
  /** 发起方会话（from 身份由此铸造，不由模型自报）。 */
  sourceSessionId: string;
  /**
   * 发起轮的工单身份（工单轮里发消息才有）：全局台账据它找回队（批次）与
   * 队长会话；缺席 = 发起方不在任何队里 → 队内校验失败。
   */
  workOrderId?: string;
}

export interface TeamMessageResult {
  targetSessionId: string;
  /** 收件人档案名（解析后）。 */
  toName: string;
  /** 消息所属批次（看板通讯区归队键）。 */
  batchId?: string;
  /** 投递语义恒 "sent"：消息已进对方工位队列；对方读没读、回不回是另一回事。 */
  delivery: "sent";
}

/**
 * 队内直达消息端口：与 AgentDispatch 同族但轻得多——投递不建工单台账、不接
 * 回执线、不计频控在飞（一句话的唤醒，不是一张工单）；队长台账落通讯留痕行。
 */
export interface TeamMessagePort {
  send(input: TeamMessageRequest): Promise<TeamMessageResult>;
}
