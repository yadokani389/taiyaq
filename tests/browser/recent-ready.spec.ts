import { test, expect, type WebSocketRoute } from "@playwright/test";
import { FLAVORS, zeroCounts, type StaffState } from "../../src/shared";

test("staff highlights newly ready orders for one minute", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  const state: StaffState = {
    revision: 1,
    day: { id: "2026-10-31", lastNumber: 102, nextSequence: 4 },
    stock: zeroCounts(),
    soldCount: 0,
    flavors: FLAVORS.map(({ id }) => ({
      id,
      enabled: true,
      perBatch: 6,
      cookingMinutes: 5,
    })),
    orders: ["ready", "waiting", "waiting"].map((status, index) => ({
      id: String(index),
      day: "2026-10-31",
      number: 100 + index,
      sequence: index,
      status: status as "ready" | "waiting",
      prioritySequence: null,
      items: { ...zeroCounts(), tsubuan: 1 },
      allocated: zeroCounts(),
      createdAt: "2026-10-31T00:00:00Z",
      finishedAt: null,
    })),
  };
  let socket: WebSocketRoute | undefined;
  await page.route("**/api/staff/live-ticket", (route) =>
    route.fulfill({ json: { ticket: "test" } }),
  );
  await page.routeWebSocket("**/api/staff/live?*", (ws) => {
    socket = ws;
    ws.send(JSON.stringify({ type: "state", state }));
    ws.onMessage(() => ws.send("pong"));
  });
  await page.route("**/api/staff/state", (route) =>
    route.fulfill({ json: state }),
  );
  await page.addInitScript(() =>
    sessionStorage.setItem("taiyaq:staff", "test"),
  );
  await page.clock.install();
  await page.goto("/staff");
  await expect(page).toHaveTitle(/taiyaq/);
  const numbers = page.locator(".order-row");
  const newlyReady = numbers.filter({ hasText: "101" });
  await expect(numbers.filter({ hasText: "100" })).toBeVisible();
  await expect(page.locator(".freshness")).toHaveText("リアルタイム更新中");
  await expect(page.locator(".recent-ready")).toHaveCount(0);

  async function update() {
    state.revision++;
    socket?.send(JSON.stringify({ type: "state", state }));
    await page.clock.runFor(5000);
  }
  state.orders[1]!.status = "ready";
  await update();
  await expect(newlyReady).toHaveClass(/recent-ready/);
  await expect(numbers.filter({ hasText: "100" })).not.toHaveClass(
    /recent-ready/,
  );
  await expect(page.locator("vite-error-overlay")).toHaveCount(0);
  await page.locator(".orders-panel").screenshot({
    path: "/tmp/taiyaq-recent-ready-staff-orders.png",
  });
  await page.screenshot({
    path: "/tmp/taiyaq-recent-ready-staff.png",
  });
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "/tmp/taiyaq-recent-ready-staff-mobile.png",
  });
  await page.clock.runFor(54000);
  await expect(newlyReady).toHaveClass(/recent-ready/);
  await page.clock.runFor(6000);
  await expect(newlyReady).not.toHaveClass(/recent-ready/);
  await update();
  await expect(newlyReady).not.toHaveClass(/recent-ready/);

  state.orders[1]!.status = "waiting";
  await update();
  await expect(newlyReady.locator(".status-label")).toHaveText("準備中");
  state.orders[1]!.status = "ready";
  await update();
  await expect(newlyReady).toHaveClass(/recent-ready/);
  state.orders[1]!.status = "waiting";
  await update();
  await expect(page.locator(".recent-ready")).toHaveCount(0);
  expect(errors).toEqual([]);
});
