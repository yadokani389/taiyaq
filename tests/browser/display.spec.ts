import { test, expect } from "@playwright/test";

test("does not give pickup instructions when status refresh fails", async ({
  page,
}) => {
  let online = true;
  await page.route("**/api/status", (route) =>
    online
      ? route.fulfill({ json: { day: "2026-10-31", flavors: [], orders: [] } })
      : route.fulfill({ status: 503, json: { error: "offline" } }),
  );
  await page.route("**/api/orders/*", (route) =>
    route.fulfill({
      json: {
        order: {
          id: "old",
          day: "2026-10-31",
          number: 100,
          status: "ready",
          items: {},
        },
        waitMinutes: 0,
      },
    }),
  );
  await page.goto("/?order=old");
  const detail = page.locator(".your-order-detail");
  await expect(detail).toContainText("受け取り口へお越しください");
  online = false;
  await expect(detail).toContainText("状態を確認できません", {
    timeout: 10000,
  });
  await expect(detail).not.toContainText("受け取り口へお越しください");
});

test("historical orders do not promise a pickup or wait time", async ({
  page,
}) => {
  await page.route("**/api/status", (route) =>
    route.fulfill({ json: { day: "2026-10-31", flavors: [], orders: [] } }),
  );
  let status = "waiting";
  await page.route("**/api/orders/*", (route) =>
    route.fulfill({
      json: {
        order: {
          id: "previous",
          day: "2026-10-30",
          number: 100,
          status,
          items: {},
        },
        waitMinutes: 0,
      },
    }),
  );
  await page.goto("/?order=previous");
  const detail = page.locator(".your-order-detail");
  await expect(detail).toContainText("過去の営業日の注文");
  await expect(detail).not.toContainText("そろそろできます");
  status = "ready";
  await page.reload();
  await expect(detail).toContainText("過去の営業日の注文");
  await expect(detail).not.toContainText("受け取り口へお越しください");
});

test("display links to the public page with a visible QR code", async ({
  page,
  baseURL,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await page.route("**/api/status", (route) =>
    route.fulfill({
      json: {
        day: "2026-10-28",
        flavors: [],
        orders: Array.from({ length: 99 }, (_, index) => ({
          id: String(index + 1),
          number: index + 1000,
          status: index < 60 ? "ready" : "waiting",
        })),
      },
    }),
  );
  await page.goto("/display?order=example#ticket");
  await expect(page).toHaveTitle(/taiyaq/);
  await expect(
    page.getByRole("heading", { name: "お受け取りいただけます" }),
  ).toBeVisible();
  const link = page.getByRole("link", { name: /スマホで注文状況を確認/ });
  await expect(link).toHaveAttribute("href", new URL("/", baseURL).href);
  const qr = page.getByRole("img", { name: "注文状況ページのQRコード" });
  await expect(qr.locator("path").last()).toHaveAttribute("d", /M/);
  for (const viewport of [
    { width: 1920, height: 1080 },
    { width: 1440, height: 900 },
    { width: 390, height: 844 },
  ]) {
    await page.setViewportSize(viewport);
    await expect(qr).toBeInViewport({ ratio: 1 });
    await expect(
      page.locator(".waiting-numbers .number-grid > span").first(),
    ).toBeInViewport({ ratio: 1 });
    expect(
      await page.evaluate(
        () =>
          document.documentElement.scrollWidth <= innerWidth &&
          document.documentElement.scrollHeight <= innerHeight,
      ),
    ).toBe(true);
    expect(
      await page.locator(".number-grid > span").evaluateAll((numbers) =>
        numbers.every((number) => {
          const range = document.createRange();
          range.selectNodeContents(number);
          const text = range.getBoundingClientRect();
          const box = number.getBoundingClientRect();
          return text.left >= box.left && text.right <= box.right;
        }),
      ),
    ).toBe(true);
    await page.screenshot({
      path: `/tmp/taiyaq-display-qr-${viewport.width}.png`,
    });
  }
  await qr.screenshot({ path: "/tmp/taiyaq-qr.png" });
  await expect(page.locator("vite-error-overlay")).toHaveCount(0);
  await link.click();
  await expect(page).toHaveURL(new URL("/", baseURL).href);
  await expect(page.getByLabel("整理券番号で確認")).toBeVisible();
  await expect(qr).toHaveCount(0);
  expect(errors).toEqual([]);
});
