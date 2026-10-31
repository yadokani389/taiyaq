import { useEffect, useRef, useState } from "react";
import type { Order } from "../shared";

export function useRecentReady(
  day: string | undefined,
  orders: ReadonlyArray<Pick<Order, "id" | "status">> | undefined,
) {
  const previous = useRef<{ day: string; ready: Set<string> }>(undefined);
  const [recent, setRecent] = useState(() => new Map<string, number>());

  useEffect(() => {
    if (!day || !orders) return;
    const ready = new Set(
      orders.filter(({ status }) => status === "ready").map(({ id }) => id),
    );
    const before = previous.current;
    const now = Date.now();
    previous.current = { day, ready };
    setRecent((current) => {
      if (before?.day !== day) return new Map();
      const next = new Map(
        [...current].filter(([id, until]) => ready.has(id) && until > now),
      );
      for (const id of ready) {
        if (!before.ready.has(id)) next.set(id, now + 60000);
      }
      return next;
    });
  }, [day, orders]);

  useEffect(() => {
    if (!recent.size) return;
    const timer = setTimeout(
      () => {
        setRecent(
          (current) =>
            new Map([...current].filter(([, until]) => until > Date.now())),
        );
      },
      Math.max(0, Math.min(...recent.values()) - Date.now()),
    );
    return () => clearTimeout(timer);
  }, [recent]);

  return recent;
}
