import { useCallback, useEffect, useRef, useState } from "react";
import type { StaffState, StaffStreamMessage } from "../shared";
import { readJson } from "./api";

export function useStaffState(token: string) {
  const [snapshot, setSnapshot] = useState<{
    token: string;
    data: StaffState;
    updatedAt: number;
  }>();
  const [error, setError] = useState<string>();
  const [live, setLive] = useState(false);
  const refreshRef = useRef<() => void>(() => {});
  const refresh = useCallback(() => refreshRef.current(), []);

  useEffect(() => {
    if (!token) return;
    const controller = new AbortController();
    let active = true;
    let socket: WebSocket | undefined;
    let connected = false;
    let connecting = false;
    let latest: StaffState | undefined;
    let polling = false;
    let pollAgain = false;
    let failures = 0;
    let retries = 0;
    let pollTimer: ReturnType<typeof setTimeout>;
    let retryTimer: ReturnType<typeof setTimeout>;
    let heartbeatTimer: ReturnType<typeof setTimeout>;
    let healthTimer: ReturnType<typeof setTimeout>;
    let pollController: AbortController | undefined;

    setLive(false);
    setError(undefined);

    function accept(state: StaffState) {
      if (!active) return;
      if (
        latest &&
        (state.revision < latest.revision || state.day.id < latest.day.id)
      )
        return;
      latest = state;
      setSnapshot({ token, data: state, updatedAt: Date.now() });
      setError(undefined);
    }

    async function poll() {
      clearTimeout(pollTimer);
      if (!active || connected) return;
      if (polling) {
        pollAgain = true;
        return;
      }
      polling = true;
      pollAgain = false;
      pollController = new AbortController();
      try {
        const response = await fetch("/api/staff/state", {
          headers: { Authorization: `Bearer ${token}` },
          cache: "no-store",
          signal: AbortSignal.any([
            controller.signal,
            pollController.signal,
            AbortSignal.timeout(10000),
          ]),
        });
        const state = await readJson<StaffState>(response);
        if (!connected) accept(state);
        failures = 0;
      } catch (cause) {
        if (active && !connected && !pollController.signal.aborted) {
          failures++;
          setError(
            cause instanceof Error ? cause.message : "通信に失敗しました",
          );
        }
      } finally {
        polling = false;
        if (active && !connected)
          pollTimer = setTimeout(
            () => {
              void poll();
            },
            pollAgain
              ? 0
              : Math.max(
                  document.hidden ? 30000 : 2000,
                  Math.min(30000, 2000 * 2 ** failures),
                ),
          );
      }
    }

    function disconnect(current: WebSocket) {
      if (!active || socket !== current) return;
      const wasConnected = connected;
      socket = undefined;
      connected = false;
      setLive(false);
      clearTimeout(heartbeatTimer);
      clearTimeout(healthTimer);
      current.close();
      if (wasConnected) void poll();
      retry();
    }

    function retry() {
      clearTimeout(retryTimer);
      if (!active) return;
      const delay = Math.min(30000, 1000 * 2 ** Math.min(retries++, 5));
      retryTimer = setTimeout(
        () => {
          void connect();
        },
        Math.min(30000, delay * (0.8 + Math.random() * 0.4)),
      );
    }

    function heartbeat(current: WebSocket) {
      clearTimeout(healthTimer);
      clearTimeout(heartbeatTimer);
      heartbeatTimer = setTimeout(() => {
        if (socket !== current) return;
        current.send("ping");
        healthTimer = setTimeout(() => disconnect(current), 10000);
      }, 25000);
    }

    async function connect() {
      clearTimeout(retryTimer);
      if (!active || socket || connecting) return;
      connecting = true;
      try {
        const response = await fetch("/api/staff/live-ticket", {
          method: "POST",
          headers: { Authorization: `Bearer ${token}` },
          signal: AbortSignal.any([
            controller.signal,
            AbortSignal.timeout(10000),
          ]),
        });
        const { ticket } = await readJson<{ ticket: string }>(response);
        if (!active) return;
        const url = new URL("/api/staff/live", location.href);
        url.protocol = location.protocol === "https:" ? "wss:" : "ws:";
        url.searchParams.set("ticket", ticket);
        const current = new WebSocket(url);
        socket = current;
        healthTimer = setTimeout(() => disconnect(current), 10000);
        current.onmessage = (event) => {
          if (!active || socket !== current) return;
          try {
            if (event.data !== "pong") {
              const message = JSON.parse(
                event.data as string,
              ) as StaffStreamMessage;
              if (message.type !== "state")
                throw new Error("更新データが正しくありません");
              accept(message.state);
              connected = true;
              retries = 0;
              setLive(true);
              clearTimeout(pollTimer);
              pollController?.abort();
            }
            heartbeat(current);
          } catch {
            disconnect(current);
          }
        };
        current.onclose = () => disconnect(current);
        current.onerror = () => disconnect(current);
      } catch {
        retry();
      } finally {
        connecting = false;
      }
    }

    function refreshState() {
      if (connected && socket?.readyState === WebSocket.OPEN) {
        socket.send("sync");
        clearTimeout(healthTimer);
        clearTimeout(heartbeatTimer);
        healthTimer = setTimeout(() => {
          if (socket) disconnect(socket);
        }, 10000);
      } else {
        void poll();
        void connect();
      }
    }

    function resume() {
      if (!document.hidden) refreshState();
    }

    refreshRef.current = refreshState;
    document.addEventListener("visibilitychange", resume);
    window.addEventListener("online", resume);
    void poll();
    void connect();
    return () => {
      active = false;
      refreshRef.current = () => {};
      controller.abort();
      pollController?.abort();
      clearTimeout(pollTimer);
      clearTimeout(retryTimer);
      clearTimeout(heartbeatTimer);
      clearTimeout(healthTimer);
      socket?.close();
      document.removeEventListener("visibilitychange", resume);
      window.removeEventListener("online", resume);
    };
  }, [token]);

  return {
    data: snapshot?.token === token ? snapshot.data : undefined,
    updatedAt: snapshot?.token === token ? snapshot.updatedAt : undefined,
    error,
    live,
    refresh,
  };
}
