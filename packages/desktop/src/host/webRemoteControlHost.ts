import { HostMessageTypes, HostResponseTypes } from "@zcode/shared";
import { createHttpServer } from "@zcode/server";
import type { ServiceCollection } from "@zcode/services";

/**
 * Web 远程控制的 Host 侧承接：main 决定端口/token 后发消息过来，这里把
 * createHttpServer 挂到窗口 Host 的同一份 ServiceCollection 上——手机连到的
 * 就是桌面正在用的这个工作区引擎（token 鉴权/静态托管/web-remote-replayable
 * 通道都是 packages/server 现成机制）。
 */

type HttpServerLike = ReturnType<typeof createHttpServer>;

let activeServer: HttpServerLike | null = null;

const { parentPort } = process;

interface WebRemoteControlHostLogger {
  info: (...args: unknown[]) => void;
  warn: (...args: unknown[]) => void;
}

interface StartMessage {
  requestId: string;
  port: number;
  token: string;
  staticRoot: string;
}

function reply(
  requestId: string,
  ok: boolean,
  error: string | undefined,
  logger: WebRemoteControlHostLogger,
): void {
  parentPort?.postMessage({
    type: HostResponseTypes.WebRemoteControlState,
    requestId,
    ok,
    ...(error ? { error } : {}),
  });
  if (!ok) logger.warn("[web-remote-host] 服务启动失败", { error });
}

export function startWebRemoteControlHost(
  message: StartMessage,
  services: ServiceCollection | null,
  logger: WebRemoteControlHostLogger,
): void {
  if (!services) {
    reply(message.requestId, false, "Host services 尚未初始化", logger);
    return;
  }
  stopWebRemoteControlHost(logger);
  try {
    activeServer = createHttpServer(services, message.port, {
      serverId: "web-remote-control",
      name: "web-remote-control",
      authToken: message.token,
      host: "0.0.0.0",
      staticRoot: message.staticRoot,
      spaFallback: true,
    });
    reply(message.requestId, true, undefined, logger);
    logger.info("[web-remote-host] 服务已挂起", { port: message.port });
  } catch (error) {
    activeServer = null;
    reply(
      message.requestId,
      false,
      error instanceof Error ? error.message : String(error),
      logger,
    );
  }
}

export function stopWebRemoteControlHost(logger: WebRemoteControlHostLogger): void {
  const server = activeServer;
  activeServer = null;
  if (!server) return;
  try {
    server.close();
    logger.info("[web-remote-host] 服务已停止");
  } catch (error) {
    logger.warn("[web-remote-host] 停止失败", {
      error: error instanceof Error ? error.message : String(error),
    });
  }
}
