import { DurableObject } from "cloudflare:workers";
import type { StaffStreamMessage } from "../shared";
import { loadState } from "./store";

export class StaffLive extends DurableObject<{ DB: D1Database }> {
  constructor(ctx: DurableObjectState, env: { DB: D1Database }) {
    super(ctx, env);
    ctx.setWebSocketAutoResponse(
      new WebSocketRequestResponsePair("ping", "pong"),
    );
  }

  async fetch() {
    return this.ctx.blockConcurrencyWhile(async () => {
      const state = await loadState(this.env.DB);
      const { 0: client, 1: server } = new WebSocketPair();
      this.ctx.acceptWebSocket(server);
      server.send(
        JSON.stringify({ type: "state", state } satisfies StaffStreamMessage),
      );
      await this.scheduleMidnight();
      return new Response(null, { status: 101, webSocket: client });
    });
  }

  async broadcast() {
    return this.ctx.blockConcurrencyWhile(async () => {
      const sockets = this.ctx.getWebSockets();
      if (!sockets.length) return;
      const state = await loadState(this.env.DB);
      const message = JSON.stringify({
        type: "state",
        state,
      } satisfies StaffStreamMessage);
      for (const socket of sockets) {
        try {
          socket.send(message);
        } catch {
          socket.close(1011, "再接続してください");
        }
      }
    });
  }

  async webSocketMessage(socket: WebSocket, message: string | ArrayBuffer) {
    if (message !== "sync") {
      socket.close(1008, "未対応のメッセージです");
      return;
    }
    await this.ctx.blockConcurrencyWhile(async () => {
      const state = await loadState(this.env.DB);
      socket.send(
        JSON.stringify({ type: "state", state } satisfies StaffStreamMessage),
      );
    });
  }

  webSocketClose(socket: WebSocket) {
    socket.close();
  }

  webSocketError(socket: WebSocket) {
    socket.close(1011, "再接続してください");
  }

  async alarm() {
    await this.broadcast();
    if (this.ctx.getWebSockets().length) await this.scheduleMidnight();
  }

  private async scheduleMidnight() {
    const day = 86400000;
    const offset = 9 * 3600000;
    const midnight =
      (Math.floor((Date.now() + offset) / day) + 1) * day - offset;
    await this.ctx.storage.setAlarm(midnight);
  }
}
