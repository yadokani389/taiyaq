import { Hono } from "hono";
import { bearerAuth as authenticate } from "hono/bearer-auth";
import { bodyLimit } from "hono/body-limit";
import { secureHeaders } from "hono/secure-headers";
import { HTTPException } from "hono/http-exception";
import { validator } from "hono/validator";
import * as v from "valibot";
import { FLAVORS, type PublicState } from "../shared";
import { commandRequestSchema } from "../validation";
import { estimateWait, UserError } from "./domain";
import { execute, history, listDays, loadState, orderDetails } from "./store";

type Bindings = { DB: D1Database; STAFF_API_TOKEN: string; ASSETS?: Fetcher };
const app = new Hono<{ Bindings: Bindings }>();
app.use("*", secureHeaders());
app.use("/api/*", async (c, next) => {
  c.header("Cache-Control", "no-store");
  await next();
});
app.use("/api/*", bodyLimit({ maxSize: 8192 }));
app.use("/api/staff/*", async (c, next) => {
  const token = c.env.STAFF_API_TOKEN;
  if (!token) throw new UserError("スタッフ認証が設定されていません", 503);
  return authenticate<{ Bindings: Bindings }>({ token })(c, next);
});
app.onError((error, c) => {
  if (error instanceof UserError)
    return c.json({ error: error.message }, error.status);
  if (error instanceof HTTPException)
    return c.json({ error: error.message }, error.status);
  if (error instanceof SyntaxError)
    return c.json({ error: "JSONの形式が正しくありません" }, 400);
  console.error("API request failed", error);
  return c.json({ error: "処理に失敗しました。同じ操作を再送信できます" }, 500);
});

const routes = app
  .get("/api/status", async (c) => {
    const state = await loadState(c.env.DB);
    const open = state.day && !state.day.closedAt;
    const result: PublicState = {
      day: open ? state.day!.id : null,
      flavors: state.flavors.map((flavor) => ({
        id: flavor.id,
        enabled: Boolean(open && flavor.enabled),
        waitMinutes:
          open && flavor.enabled ? estimateWait(state, flavor.id) : null,
      })),
      orders: open
        ? state.orders.map(({ id, number, status }) => ({ id, number, status }))
        : [],
    };
    return c.json(result);
  })
  .get("/api/orders/:id", async (c) => {
    const order = await orderDetails(c.env.DB, c.req.param("id"));
    if (!order) throw new UserError("注文が見つかりません", 404);
    const state = await loadState(c.env.DB);
    const waitMinutes =
      order.status === "waiting" && state.day?.id === order.day
        ? Math.max(
            ...FLAVORS.filter(
              ({ id }) => order.items[id] > order.allocated[id],
            ).map(({ id }) => estimateWait(state, id, order.sequence) ?? 0),
          )
        : 0;
    const { id, day, number, status, items } = order;
    return c.json({ order: { id, day, number, status, items }, waitMinutes });
  })
  .get("/api/staff/state", async (c) => c.json(await loadState(c.env.DB)))
  .get("/api/staff/days", async (c) => c.json(await listDays(c.env.DB)))
  .get("/api/staff/history/:day", async (c) => {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(c.req.param("day")))
      throw new UserError("営業日が正しくありません");
    return c.json(await history(c.env.DB, c.req.param("day")));
  })
  .post(
    "/api/staff/commands",
    validator("json", (value, c) => {
      const parsed = v.safeParse(commandRequestSchema, value);
      if (!parsed.success)
        return c.json({ error: parsed.issues[0].message }, 400);
      return parsed.output;
    }),
    async (c) => {
      const { operationId, command } = c.req.valid("json");
      return c.json(await execute(c.env.DB, operationId, command));
    },
  );
app.notFound(async (c) => {
  if (c.req.path.startsWith("/api/") || !c.env.ASSETS)
    return c.json({ error: "ページが見つかりません" }, 404);
  const response = await c.env.ASSETS.fetch(c.req.raw);
  return new Response(response.body, response);
});
export type Api = typeof routes;
export default app;
