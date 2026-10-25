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
  await page.keyboard.press("1");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("1");
  await page.keyboard.press("Enter");
  await expect(page.locator(".notice")).toContainText("1番で受け付けました");
  await page.keyboard.press("1");
  await page.keyboard.press("NumpadEnter");
  await expect(page.locator(".notice")).toContainText("2番で受け付けました");
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
  await expect(page.locator(".notice")).toContainText("3番で受け付けました");
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
  const publicPage = await context.newPage();
  await publicPage.goto("/");
  const ticket = publicPage.getByLabel("整理券番号で確認", { exact: true });
  await ticket.fill("100");
  await publicPage.getByRole("button", { name: "確認", exact: true }).click();
  expect(
    await ticket.evaluate(
      (input: HTMLInputElement) => input.validity.rangeOverflow,
    ),
  ).toBe(true);
  await ticket.fill("3");
  await publicPage.getByRole("button", { name: "確認", exact: true }).click();
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
  const display = await context.newPage();
  await display.goto("/display");
  await expect(display.locator(".ready-numbers .number-grid")).toContainText(
    "03",
  );
  await display.screenshot({ path: "/tmp/taiyaq-display.png", fullPage: true });
  await page.bringToFront();
  await page.keyboard.press("/");
  const search = page.getByRole("textbox", { name: "整理券番号で検索" });
  const receive = page.getByRole("button", { name: "受け渡し ↵", exact: true });
  await page.keyboard.type("1");
  await expect(receive).toBeDisabled();
  await page.keyboard.press("Enter");
  await expect(search).toHaveValue("1");
  await page.keyboard.press("Escape");
  await expect(search).not.toBeFocused();
  await page.keyboard.press("/");
  await page.keyboard.type("3");
  await expect(receive).toBeEnabled();
  await page.screenshot({ path: "/tmp/taiyaq-receiving.png", fullPage: true });
  page.once("dialog", (dialog) => {
    void dialog.dismiss();
  });
  await page.keyboard.press("Enter");
  await expect(search).toHaveValue("3");
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
  await page.keyboard.type("1");
  await expect(search).toHaveValue("1");
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
  expect(errors).toEqual([]);
});
