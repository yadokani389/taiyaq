import { test, expect, type Page, type WebSocketRoute } from "@playwright/test";
import { readFileSync } from "node:fs";
import type { CommandResult, StaffState } from "../../src/shared";

const token = readFileSync(".dev.vars", "utf8")
  .match(/^STAFF_API_TOKEN=(.+)$/m)?.[1]
  ?.trim();
if (!token) throw new Error(".dev.varsにSTAFF_API_TOKENを設定してください");
const headers = { Authorization: `Bearer ${token}` };

async function login(page: Page) {
  await page.goto("/staff");
  await page.getByLabel("スタッフ用トークン").fill(token!);
  await page.getByRole("button", { name: "ログイン", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "注文受付", exact: true }),
  ).toBeVisible();
}

test("pushes complete staff state to multiple tabs without polling", async ({
  page,
  context,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  let reads = 0;
  page.on("request", (request) => {
    if (new URL(request.url()).pathname === "/api/staff/state") reads++;
  });
  await login(page);
  await expect(page.locator(".freshness")).toHaveText("リアルタイム更新中");
  const other = await context.newPage();
  await login(other);
  await expect(other.locator(".freshness")).toHaveText("リアルタイム更新中");
  const state = (await (
    await context.request.get("/api/staff/state", { headers })
  ).json()) as StaffState;
  const batch = state.flavors[0]!.perBatch === 7 ? 6 : 7;
  const response = await context.request.post("/api/staff/commands", {
    headers,
    data: {
      operationId: crypto.randomUUID(),
      command: {
        type: "flavor",
        flavor: { ...state.flavors[0]!, perBatch: batch },
      },
    },
  });
  expect(response.ok()).toBe(true);
  for (const tab of [page, other]) {
    await tab.getByRole("button", { name: /販売設定/ }).click();
    await expect(
      tab
        .locator(".flavor-setting")
        .filter({ hasText: "粒あん" })
        .getByLabel("1回の目安（個）"),
    ).toHaveValue(String(batch));
  }
  const before = reads;
  await page.clock.install();
  await page.clock.fastForward(5000);
  expect(reads).toBe(before);
  await page.getByRole("button", { name: /注文受付/ }).click();
  await expect(page).toHaveTitle(/taiyaq/);
  await expect(page.locator("vite-error-overlay")).toHaveCount(0);
  await page.screenshot({ path: "/tmp/taiyaq-staff-live.png", fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "/tmp/taiyaq-staff-live-mobile.png",
    fullPage: true,
  });
  expect(errors).toEqual([]);
  await page.setViewportSize({ width: 1440, height: 1000 });
  const created = await context.request.post("/api/staff/commands", {
    headers,
    data: {
      operationId: crypto.randomUUID(),
      command: { type: "order", day: state.day.id, items: { tsubuan: 1 } },
    },
  });
  expect(created.ok()).toBe(true);
  const { order } = (await created.json()) as CommandResult;
  await page.getByRole("button", { name: /履歴/ }).click();
  const row = page
    .locator(".history-row")
    .filter({ hasText: `${order!.number}番` });
  await expect(row).toBeVisible();
  let historyReads = 0;
  page.on("request", (request) => {
    if (new URL(request.url()).pathname.startsWith("/api/staff/history/"))
      historyReads++;
  });
  const cancelled = await context.request.post("/api/staff/commands", {
    headers,
    data: {
      operationId: crypto.randomUUID(),
      command: { type: "cancel", day: state.day.id, orderId: order!.id },
    },
  });
  expect(cancelled.ok()).toBe(true);
  await expect(row).toContainText("キャンセル済み");
  const historyBefore = historyReads;
  await page.clock.fastForward(5000);
  expect(historyReads).toBe(historyBefore);
});

test("falls back to HTTP, backs off failures, refreshes on resume and reconnects automatically", async ({
  page,
  context,
}) => {
  let allow = true;
  let stream: WebSocketRoute | undefined;
  await page.routeWebSocket("**/api/staff/live?*", async (socket) => {
    if (allow) {
      stream = socket;
      socket.connectToServer();
    } else await socket.close();
  });
  let reads = 0;
  let failed = false;
  const timestamps: number[] = [];
  await page.route("**/api/staff/state", async (route) => {
    reads++;
    timestamps.push(Date.now());
    if (failed)
      await route.fulfill({ status: 503, json: { error: "通信テスト" } });
    else await route.continue();
  });
  await login(page);
  await expect(page.locator(".freshness")).toHaveText("リアルタイム更新中");
  allow = false;
  await stream!.close();
  await expect(page.locator(".freshness")).toContainText("定期更新中");
  const initial = reads;
  await expect.poll(() => reads, { timeout: 5000 }).toBeGreaterThan(initial);
  expect(timestamps.at(-1)! - timestamps.at(-2)!).toBeGreaterThanOrEqual(1800);
  const state = (await (
    await context.request.get("/api/staff/state", { headers })
  ).json()) as StaffState;
  const nextBatch = state.flavors[0]!.perBatch === 6 ? 7 : 6;
  await context.request.post("/api/staff/commands", {
    headers,
    data: {
      operationId: crypto.randomUUID(),
      command: {
        type: "flavor",
        flavor: { ...state.flavors[0]!, perBatch: nextBatch },
      },
    },
  });
  await page.getByRole("button", { name: /販売設定/ }).click();
  await expect(
    page
      .locator(".flavor-setting")
      .filter({ hasText: "粒あん" })
      .getByLabel("1回の目安（個）"),
  ).toHaveValue(String(nextBatch));
  await page.screenshot({
    path: "/tmp/taiyaq-staff-polling.png",
    fullPage: true,
  });
  failed = true;
  await expect(page.locator(".freshness")).toHaveText("更新できていません");
  const failedReads = reads;
  await expect
    .poll(() => reads, { timeout: 7000 })
    .toBeGreaterThan(failedReads);
  expect(timestamps.at(-1)! - timestamps.at(-2)!).toBeGreaterThanOrEqual(3800);
  failed = false;
  await page.evaluate(() => window.dispatchEvent(new Event("online")));
  await expect(page.locator(".freshness")).toContainText("定期更新中");
  allow = true;
  await page.evaluate(() => window.dispatchEvent(new Event("online")));
  await expect(page.locator(".freshness")).toHaveText("リアルタイム更新中");
  const after = reads;
  await page.clock.install();
  await page.clock.fastForward(5000);
  expect(reads).toBe(after);
});

test("polls every 30 seconds while hidden and immediately on returning", async ({
  page,
}) => {
  await page.clock.install({ time: new Date("2040-01-01T00:00:00Z") });
  await page.clock.pauseAt(new Date("2040-01-01T00:00:01Z"));
  await page.routeWebSocket("**/api/staff/live?*", async (socket) => {
    await socket.close();
  });
  let reads = 0;
  page.on("request", (request) => {
    if (new URL(request.url()).pathname === "/api/staff/state") reads++;
  });
  await login(page);
  await expect(page.locator(".freshness")).toContainText("定期更新中");
  const initial = reads;
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", {
      configurable: true,
      value: true,
    });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await page.clock.runFor(2100);
  await expect.poll(() => reads).toBe(initial + 1);
  await expect(page.locator(".freshness")).toContainText("9:00:03");
  await page.clock.runFor(29000);
  expect(reads).toBe(initial + 1);
  await page.clock.runFor(1100);
  await expect.poll(() => reads).toBe(initial + 2);
  await expect(page.locator(".freshness")).toContainText("9:00:33");
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", {
      configurable: true,
      value: false,
    });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await expect.poll(() => reads).toBe(initial + 3);
});

test("detects an unresponsive WebSocket and resumes HTTP polling", async ({
  page,
}) => {
  await page.clock.install({ time: new Date("2040-01-01T00:00:00Z") });
  await page.clock.pauseAt(new Date("2040-01-01T00:00:01Z"));
  let allow = true;
  let pings = 0;
  await page.routeWebSocket("**/api/staff/live?*", async (socket) => {
    if (!allow) {
      await socket.close();
      return;
    }
    const server = socket.connectToServer();
    socket.onMessage((message) => {
      if (message === "ping") pings++;
      else server.send(message);
    });
  });
  await login(page);
  await expect(page.locator(".freshness")).toHaveText("リアルタイム更新中");
  allow = false;
  await page.clock.runFor(25000);
  await expect.poll(() => pings).toBe(1);
  await page.clock.runFor(10001);
  await expect(page.locator(".freshness")).toContainText("定期更新中");
});
