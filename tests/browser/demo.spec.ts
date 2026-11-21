import { expect, test } from "@playwright/test";

test("staff can serve a demo order across tabs without connecting to the API", async ({
  page,
  context,
  browser,
}) => {
  const backendRequests: string[] = [];
  context.on("request", (request) => {
    if (new URL(request.url()).pathname.startsWith("/api/"))
      backendRequests.push(request.url());
  });
  const sockets: string[] = [];
  context.on("page", (tab) =>
    tab.on("websocket", (socket) => sockets.push(socket.url())),
  );
  page.on("websocket", (socket) => sockets.push(socket.url()));
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: /ありがとうございました/ }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: /店頭モニターデモ/ }),
  ).toHaveAttribute("href", "/display");
  await expect(
    page.getByRole("link", { name: /来場者向けデモ/ }),
  ).toHaveAttribute("href", "/order");
  await page.getByRole("link", { name: /スタッフ操作デモ/ }).click();
  const monitor = await context.newPage();
  await monitor.goto("/display");
  const visitor = await context.newPage();
  await visitor.goto("/order");
  await expect(
    visitor.getByRole("button", { name: "101番 準備中" }),
  ).toBeVisible();
  const order = page.getByRole("region", { name: "注文受付", exact: true });
  await order.getByRole("button", { name: "粒あん 1個", exact: true }).click();
  await order.getByRole("button", { name: /注文を登録/ }).click();
  await expect(
    visitor.getByRole("button", { name: "102番 準備中" }),
  ).toBeVisible();
  await visitor.getByLabel("整理券番号で確認").fill("102");
  await visitor.getByRole("button", { name: "確認", exact: true }).click();
  await expect(
    visitor
      .locator(".your-order-detail")
      .getByRole("heading", { name: "準備中", exact: true }),
  ).toBeVisible();
  const production = page.getByRole("region", {
    name: "焼き上がり",
    exact: true,
  });
  await production
    .getByRole("button", { name: "粒あん 1個", exact: true })
    .click();
  await production.getByRole("button", { name: /焼き上がりを登録/ }).click();
  await expect(
    monitor.locator(".ready-numbers").getByText("102", { exact: true }),
  ).toBeVisible();
  await expect(
    visitor.getByRole("heading", { name: "受け取り可能", exact: true }),
  ).toBeVisible();
  page.once("dialog", (dialog) => dialog.accept());
  await page.locator("#ticket-search").fill("102");
  await page.getByRole("button", { name: /^受け渡し ↵$/ }).click();
  await expect(
    visitor.getByRole("heading", { name: "受け渡し済み", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: /履歴/ }).click();
  await expect(
    page.locator(".history-row").filter({ hasText: "102番" }),
  ).toContainText("受け渡し済み");
  const other = await browser.newPage();
  try {
    await other.goto(visitor.url().split("?")[0]!);
    await expect(
      other.getByRole("button", { name: "100番 受け取り可能" }),
    ).toBeVisible();
    await expect(other.getByRole("button", { name: /^102番/ })).toHaveCount(0);
  } finally {
    await other.close();
  }
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "デモを初期状態に戻す" }).click();
  await expect(page.locator("#entry-order .submit")).toContainText("102番");
  await expect(
    visitor.getByRole("heading", { name: "受け渡し済み", exact: true }),
  ).toHaveCount(0);
  expect(backendRequests).toEqual([]);
  expect(sockets).toEqual([]);
});
