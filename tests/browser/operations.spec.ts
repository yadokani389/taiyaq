import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import { type State } from "../../src/shared";

test("keyboard and mouse orders, flavor FIFO, retry, receiving and public screens", async ({
  page,
  context,
}) => {
  const token = readFileSync(".dev.vars", "utf8")
    .match(/^STAFF_API_TOKEN=(.+)$/m)?.[1]
    ?.trim();
  if (!token) throw new Error(".dev.varsにSTAFF_API_TOKENを設定してください");
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/staff");
  await page.getByLabel("スタッフ用トークン").fill(token);
  await page.getByRole("button", { name: "ログイン", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "粒あん 1個", exact: true }),
  ).toBeVisible();
  for (const [key, heading] of [
    ["k", "焼き上がり"],
    ["l", "販売設定"],
    [";", "履歴"],
    ["j", "注文受付"],
  ] as const) {
    await page.keyboard.press(key);
    await expect(
      page.getByRole("heading", { name: heading, exact: true }),
    ).toBeVisible();
  }
  await page.keyboard.press("/");
  await page.keyboard.press("k");
  await expect(
    page.getByRole("heading", { name: "注文受付", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await page.keyboard.press("1");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("1");
  await page.keyboard.press("Enter");
  await expect(page.locator(".notice")).toContainText("100番で受け付けました");
  await expect(page.locator(".order-row .order-wait")).toHaveText(
    "そろそろできます",
  );
  await page.keyboard.press("1");
  await page.keyboard.press("NumpadEnter");
  await expect(page.locator(".notice")).toContainText("101番で受け付けました");
  await page.getByRole("button", { name: /販売設定/ }).click();
  await page
    .locator(".flavor-setting")
    .filter({ hasText: "栗きんとん" })
    .getByRole("button", { name: "受付停止", exact: true })
    .click();
  await expect(
    page
      .locator(".flavor-setting")
      .filter({ hasText: "栗きんとん" })
      .getByRole("button", { name: "受付中", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: /注文受付/ }).click();
  await page
    .getByRole("button", { name: "栗きんとん 1個", exact: true })
    .click();
  await page.getByRole("button", { name: /注文を登録/ }).click();
  await expect(page.locator(".notice")).toContainText("102番で受け付けました");
  await page
    .getByRole("button", { name: /焼き上がり/ })
    .first()
    .click();
  await page.getByRole("button", { name: "粒あん 1個", exact: true }).click();
  await page
    .getByRole("button", { name: "栗きんとん 1個", exact: true })
    .click();
  let dropped = false;
  await page.route("**/api/staff/commands", async (route) => {
    if (dropped) return route.continue();
    dropped = true;
    await route.fetch();
    await route.abort("failed");
  });
  await page.keyboard.press("1");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("alert")).toContainText(
    "送信結果を確認できません",
  );
  await page.getByRole("button", { name: "同じ操作を再送信" }).click();
  await expect(page.locator(".notice")).toContainText(
    "焼き上がりを登録しました",
  );
  await page.unroute("**/api/staff/commands");
  const response = await context.request.get("/api/staff/state", {
    headers: { Authorization: `Bearer ${token}` },
  });
  const state = (await response.json()) as State;
  expect(state.orders.map(({ status }) => status)).toEqual([
    "waiting",
    "waiting",
    "ready",
  ]);
  expect(state.orders[0]!.allocated.tsubuan).toBe(1);
  expect(state.orders[1]!.allocated.tsubuan).toBe(0);
  expect(state.orders[1]!.items.tsubuan).toBe(1);
  expect(state.orders[1]!.items.custard).toBe(0);
  await page.getByRole("button", { name: /注文受付/ }).click();
  await page.screenshot({ path: "/tmp/taiyaq-staff.png", fullPage: true });
  const search = page.getByRole("textbox", { name: "整理券番号で検索" });
  await search.fill("1000");
  await expect(search).toHaveValue("1000");
  await page.keyboard.press("Escape");
  const receive = page.getByRole("button", { name: "受け渡し ↵", exact: true });
  await page.keyboard.press("/");
  await page.keyboard.type("102");
  await page.keyboard.press(".");
  await expect(page.locator(".order-row.skipped")).toContainText("102");
  await expect(page.locator(".order-row.skipped .order-wait")).toHaveCount(0);
  await expect(search).toBeFocused();
  await expect(search).toHaveValue("");
  await page
    .locator(".orders-panel")
    .screenshot({ path: "/tmp/taiyaq-skipped.png" });
  const publicPage = await context.newPage();
  publicPage.on("pageerror", (error) => errors.push(error.message));
  publicPage.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await publicPage.goto("/");
  const ticket = publicPage.getByLabel("整理券番号で確認", { exact: true });
  await ticket.fill("1000");
  await publicPage.getByRole("button", { name: "確認", exact: true }).click();
  expect(
    await ticket.evaluate(
      (input: HTMLInputElement) => input.validity.rangeOverflow,
    ),
  ).toBe(false);
  await expect(publicPage.getByRole("alert")).toContainText("見つかりません");
  await ticket.fill("102");
  await publicPage.getByRole("button", { name: "確認", exact: true }).click();
  await expect(publicPage.locator(".your-order-detail")).toContainText(
    "受け取り可能",
  );
  const display = await context.newPage();
  await display.goto("/display");
  await expect(display.locator(".ready-numbers .number-grid")).toContainText(
    "102",
  );
  await display.screenshot({
    path: "/tmp/taiyaq-held-display.png",
    fullPage: true,
  });
  await page.bringToFront();
  await page.keyboard.type("102");
  await expect(
    page.getByRole("button", { name: "列に戻す ↵", exact: true }),
  ).toBeEnabled();
  await page.keyboard.press("Enter");
  await expect(page.locator(".notice")).toContainText("102番を列に戻しました");
  await expect(receive).toBeEnabled();
  await expect(search).toHaveValue("102");
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "/tmp/taiyaq-skip-mobile.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await publicPage.bringToFront();
  await expect(publicPage.locator(".your-order-detail")).toContainText(
    "受け取り可能",
  );
  await publicPage.setViewportSize({ width: 390, height: 844 });
  await publicPage.screenshot({
    path: "/tmp/taiyaq-customer.png",
    fullPage: true,
  });
  expect(
    await publicPage.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await expect(display.locator(".ready-numbers .number-grid")).toContainText(
    "102",
  );
  await display.screenshot({ path: "/tmp/taiyaq-display.png", fullPage: true });
  await page.bringToFront();
  await page.keyboard.press("/");
  await page.keyboard.type("100");
  await expect(receive).toBeDisabled();
  await page.keyboard.press("Enter");
  await expect(search).toHaveValue("100");
  await page.keyboard.press("Escape");
  await expect(search).not.toBeFocused();
  await page.keyboard.press("/");
  await page.keyboard.type("102");
  await expect(receive).toBeEnabled();
  await page.screenshot({ path: "/tmp/taiyaq-receiving.png", fullPage: true });
  page.once("dialog", (dialog) => {
    void dialog.dismiss();
  });
  await page.keyboard.press("Enter");
  await expect(search).toHaveValue("102");
  await page.keyboard.press("Tab");
  await expect(receive).toBeFocused();
  page.once("dialog", (dialog) => {
    void dialog.accept();
  });
  await page.keyboard.press("Enter");
  await expect(page.locator(".order-row.ready")).toHaveCount(0);
  await expect(search).toBeFocused();
  await expect(search).toHaveValue("");
  await expect(page.locator(".order-row")).toHaveCount(2);
  await page.keyboard.type("100");
  await expect(search).toHaveValue("100");
  await expect(page.getByLabel("粒あんの数量", { exact: true })).toHaveText(
    "0",
  );
  await page.keyboard.press("Escape");
  await publicPage.bringToFront();
  await expect(publicPage.locator(".your-order-detail")).toContainText(
    "受け渡し済み",
    {
      timeout: 35000,
    },
  );
  await page.bringToFront();
  await page.setViewportSize({ width: 390, height: 844 });
  await page
    .getByRole("button", { name: /焼き上がり/ })
    .first()
    .click();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "/tmp/taiyaq-staff-mobile.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.getByRole("button", { name: /注文受付/ }).click();
  const firstOrder = page.locator(".order-row").filter({
    has: page.locator(".ticket-number", { hasText: /^100$/ }),
  });
  const secondOrder = page.locator(".order-row").filter({
    has: page.locator(".ticket-number", { hasText: /^101$/ }),
  });
  const consoleErrors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  await secondOrder
    .getByRole("button", { name: "優先する", exact: true })
    .click();
  await expect(page.locator(".notice")).toContainText(
    "101番が受け取り可能になりました",
  );
  await expect(secondOrder.locator(".status-label")).toHaveText("受け取り可能");
  await expect(firstOrder).toContainText("確保 0");
  await firstOrder
    .getByRole("button", { name: "優先する", exact: true })
    .click();
  await expect(firstOrder.locator(".status-label")).toHaveText("優先待ち");
  await expect(secondOrder.locator(".status-label")).toHaveText("受け取り可能");
  await expect(page.locator(".order-list .ticket-number")).toHaveText([
    "101",
    "100",
  ]);
  await expect(page).toHaveURL(/\/staff$/);
  await expect(page).toHaveTitle(/taiyaq/);
  await expect(page.locator("vite-error-overlay")).toHaveCount(0);
  await page.screenshot({
    path: "/tmp/taiyaq-priority-desktop.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page
    .locator(".orders-panel")
    .screenshot({ path: "/tmp/taiyaq-priority-mobile.png" });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.keyboard.press("/");
  await page.keyboard.type("100");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  const removePriority = firstOrder.getByRole("button", {
    name: "優先を解除",
    exact: true,
  });
  await expect(removePriority).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator(".notice")).toContainText(
    "100番の優先を解除しました",
  );
  await expect(firstOrder.locator(".status-label")).toHaveText("準備中");
  await expect(search).toBeFocused();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  await expect(
    firstOrder.getByRole("button", { name: "優先する", exact: true }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(firstOrder.locator(".status-label")).toHaveText("優先待ち");
  await page.keyboard.press("/");
  await page.keyboard.press("Escape");
  await page.reload();
  await expect(firstOrder.locator(".status-label")).toHaveText("優先待ち");
  await expect(secondOrder.locator(".status-label")).toHaveText("受け取り可能");
  await publicPage.goto("/");
  await publicPage.getByLabel("整理券番号で確認", { exact: true }).fill("100");
  await publicPage.getByRole("button", { name: "確認", exact: true }).click();
  await expect(publicPage.locator(".your-order-detail")).toContainText(
    "準備中",
  );
  await expect(publicPage.locator(".your-order-detail")).not.toContainText(
    "優先待ち",
  );
  await expect(publicPage.locator(".your-order-detail")).toContainText(
    "そろそろできます",
  );
  await publicPage.setViewportSize({ width: 1440, height: 1000 });
  await expect(publicPage).toHaveTitle(/taiyaq/);
  await expect(publicPage.locator("vite-error-overlay")).toHaveCount(0);
  await publicPage.screenshot({ path: "/tmp/taiyaq-public-wait-desktop.png" });
  await publicPage.setViewportSize({ width: 390, height: 844 });
  await publicPage.screenshot({ path: "/tmp/taiyaq-public-wait-soon.png" });
  await display.reload();
  await expect(display.locator(".ready-numbers .number-grid")).toContainText(
    "101",
  );
  await expect(
    display.locator(".ready-numbers .number-grid"),
  ).not.toContainText("100");
  await expect(firstOrder.locator(".order-wait")).toHaveText(
    "そろそろできます",
  );
  await expect(secondOrder.locator(".order-wait")).toHaveCount(0);
  await page.getByRole("button", { name: /注文受付/ }).click();
  await page.getByRole("button", { name: "粒あん 3個", exact: true }).click();
  await page.keyboard.press("9");
  await page.keyboard.press("ArrowRight");
  await page.getByRole("button", { name: /注文を登録/ }).click();
  await expect(page.locator(".notice")).toContainText("103番で受け付けました");
  const laterOrder = page.locator(".order-row").filter({
    has: page.locator(".ticket-number", { hasText: /^103$/ }),
  });
  await expect(laterOrder.locator(".order-wait")).toHaveText("約15分後");
  await publicPage.reload();
  await publicPage.getByLabel("整理券番号で確認", { exact: true }).fill("103");
  await publicPage.getByRole("button", { name: "確認", exact: true }).click();
  await expect(publicPage.locator(".your-order-detail")).toContainText(
    "約15分後",
  );
  expect(
    await publicPage.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await publicPage.screenshot({ path: "/tmp/taiyaq-public-wait-later.png" });
  await page.screenshot({
    path: "/tmp/taiyaq-staff-wait-desktop.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.locator(".orders-panel").scrollIntoViewIfNeeded();
  await expect(firstOrder.locator(".order-wait")).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page
    .locator(".orders-panel")
    .screenshot({ path: "/tmp/taiyaq-staff-wait-mobile.png" });
  await page
    .getByRole("button", { name: /焼き上がり/ })
    .first()
    .click();
  await page.getByRole("button", { name: "粒あん 2個", exact: true }).click();
  await page.getByRole("button", { name: /焼き上がりを登録/ }).click();
  await expect(laterOrder.locator(".order-wait")).toHaveText(
    "そろそろできます",
  );
  await publicPage.reload();
  await expect(publicPage.locator(".your-order-detail")).toContainText(
    "そろそろできます",
  );
  await page.getByRole("button", { name: /履歴/ }).click();
  await expect(page.locator(".day-list")).toContainText(state.day.id);
  await expect(page.locator(".history-row")).toHaveCount(4);
  await expect(page.locator(".history-row")).toContainText([
    "103番",
    "102番",
    "101番",
    "100番",
  ]);
  expect(consoleErrors).toEqual([]);
  expect(errors).toEqual([]);
});
