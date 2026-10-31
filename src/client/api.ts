import { useEffect, useReducer, useState } from "react";
import { hc } from "hono/client";
import type { Api } from "../server";
import type { CommandRequest, CommandResult } from "../shared";

const client = hc<Api>("/");
export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
  }
}

export async function readJson<T>(
  response: Pick<Response, "json" | "ok" | "status">,
): Promise<T> {
  let body: unknown;
  try {
    body = await response.json();
  } catch {
    throw new ApiError(
      "サーバーからの応答を確認できません",
      response.status >= 400 ? response.status : 500,
    );
  }
  if (!response.ok)
    throw new ApiError(
      body &&
        typeof body === "object" &&
        "error" in body &&
        typeof body.error === "string"
        ? body.error
        : "処理に失敗しました",
      response.status,
    );
  return body as T;
}

export async function sendCommand(
  request: CommandRequest,
  token: string,
): Promise<CommandResult> {
  return readJson(
    await client.api.staff.commands.$post(
      { json: request },
      {
        headers: { Authorization: `Bearer ${token}` },
        init: { signal: AbortSignal.timeout(15000) },
      },
    ),
  );
}

export function usePolling<T>(
  path: string | null,
  token?: string,
  {
    refreshKey,
    interval = 5000,
  }: { refreshKey?: string; interval?: number | false } = {},
) {
  const [data, setData] = useState<T>();
  const [loadedPath, setLoadedPath] = useState<string>();
  const [error, setError] = useState<string>();
  const [updatedAt, setUpdatedAt] = useState<number>();
  const [version, refresh] = useReducer((value: number) => value + 1, 0);
  useEffect(() => {
    if (!path) {
      setData(undefined);
      return;
    }
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    let active = true;
    let failures = 0;
    async function poll() {
      try {
        const response = await fetch(path!, {
          headers: token ? { Authorization: `Bearer ${token}` } : {},
          cache: "no-store",
          signal: AbortSignal.any([
            controller.signal,
            AbortSignal.timeout(10000),
          ]),
        });
        const next = await readJson<T>(response);
        if (active) {
          setData(next);
          setLoadedPath(path!);
          setError(undefined);
          setUpdatedAt(Date.now());
          failures = 0;
        }
      } catch (error) {
        if (active) {
          failures++;
          setError(
            error instanceof Error ? error.message : "通信に失敗しました",
          );
        }
      } finally {
        if (active && (interval !== false || failures > 0))
          timer = setTimeout(
            () => {
              void poll();
            },
            document.hidden
              ? 30000
              : Math.min(
                  30000,
                  (interval === false ? 5000 : interval) * 2 ** failures,
                ),
          );
      }
    }
    void poll();
    return () => {
      active = false;
      controller.abort();
      clearTimeout(timer);
    };
  }, [path, token, version, refreshKey, interval]);
  return {
    data: loadedPath === path ? data : undefined,
    error,
    updatedAt,
    refresh,
  };
}
