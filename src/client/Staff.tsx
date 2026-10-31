import { useEffect, useState } from "react";
import * as v from "valibot";
import {
  FLAVORS,
  STATUS_LABELS,
  compareAllocationOrder,
  estimateOrderWait,
  total,
  zeroCounts,
  type Command,
  type CommandRequest,
  type Counts,
  type Day,
  type Flavor,
  type Order,
  type State,
} from "../shared";
import { commandRequestSchema } from "../validation";
import { ApiError, sendCommand, usePolling } from "./api";
import { QuantityPicker } from "./QuantityPicker";
import { useStaffState } from "./useStaffState";
import { useRecentReady } from "./useRecentReady";

type View = "order" | "settings" | "history";
const views: Array<{ id: View; label: string; key: string }> = [
  { id: "order", label: "注文受付", key: "J" },
  { id: "settings", label: "販売設定", key: "K" },
  { id: "history", label: "履歴", key: "L" },
];

function storedRequest(): CommandRequest | undefined {
  try {
    const result = v.safeParse(
      commandRequestSchema,
      JSON.parse(sessionStorage.getItem("taiyaq:pending") ?? "null"),
    );
    return result.success ? result.output : undefined;
  } catch {
    return undefined;
  }
}

export function Staff() {
  const [token, setToken] = useState(
    () => sessionStorage.getItem("taiyaq:staff") ?? "",
  );
  const [login, setLogin] = useState("");
  const [view, setView] = useState<View>("order");
  const [orderItems, setOrderItems] = useState<Counts>(zeroCounts);
  const [productionItems, setProductionItems] = useState<Counts>(zeroCounts);
  const [formVersion, setFormVersion] = useState({ order: 0, production: 0 });
  const [pending, setPending] = useState<CommandRequest | undefined>(
    storedRequest,
  );
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");
  const poll = useStaffState(token);
  const state = poll.data;
  const day = state?.day.id;
  const recentReady = useRecentReady(day, state?.orders);
  const disabled = busy || Boolean(pending);
  const selectedOrder = state?.orders.find(
    ({ number }) => number === Number(search),
  );
  const canSkip =
    selectedOrder?.status === "waiting" || selectedOrder?.status === "ready";

  useEffect(() => {
    function navigate(event: KeyboardEvent) {
      if (
        event.isComposing ||
        event.repeat ||
        event.ctrlKey ||
        event.metaKey ||
        event.altKey ||
        (event.target as HTMLElement | null)?.closest(
          'input, textarea, select, [contenteditable="true"]',
        )
      )
        return;
      const next = views.find(
        ({ key }) => key.toLowerCase() === event.key.toLowerCase(),
      );
      if (next) {
        event.preventDefault();
        setView(next.id);
      }
      if (event.key === "/") {
        event.preventDefault();
        document.querySelector<HTMLInputElement>("#ticket-search")?.focus();
      }
    }
    document.addEventListener("keydown", navigate);
    return () => document.removeEventListener("keydown", navigate);
  }, []);

  async function transmit(request: CommandRequest) {
    if (busy) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const result = await sendCommand(request, token);
      sessionStorage.removeItem("taiyaq:pending");
      setPending(undefined);
      if (request.command.type === "order") {
        setOrderItems(zeroCounts());
        setFormVersion((version) => ({ ...version, order: version.order + 1 }));
        setNotice(`${result.order!.number}番で受け付けました`);
      } else if (request.command.type === "production") {
        setProductionItems(zeroCounts());
        setFormVersion((version) => ({
          ...version,
          production: version.production + 1,
        }));
        setNotice("焼き上がりを登録しました");
      } else if (request.command.type === "skip") {
        setNotice(`${result.order!.number}番を保留にしました`);
      } else if (request.command.type === "resume") {
        setNotice(`${result.order!.number}番を列に戻しました`);
      } else if (request.command.type === "prioritize") {
        setNotice(
          result.order!.status === "ready"
            ? `${result.order!.number}番が受け取り可能になりました`
            : `${result.order!.number}番を優先にしました`,
        );
      } else if (request.command.type === "unprioritize") {
        setNotice(`${result.order!.number}番の優先を解除しました`);
      } else setNotice("保存しました");
      if (request.command.type === "adjust")
        document.querySelector<HTMLFormElement>(".adjustment")?.reset();
      poll.refresh();
    } catch (cause) {
      if (
        cause instanceof ApiError &&
        cause.status >= 400 &&
        cause.status < 500
      ) {
        sessionStorage.removeItem("taiyaq:pending");
        setPending(undefined);
        setError(`${cause.message}（未登録）`);
      } else setError("送信結果を確認できません。同じ操作を再送信してください");
    } finally {
      setBusy(false);
    }
  }
  function act(command: Command) {
    if (disabled) return;
    const request = { operationId: crypto.randomUUID(), command };
    sessionStorage.setItem("taiyaq:pending", JSON.stringify(request));
    setPending(request);
    void transmit(request);
  }
  function submit(type: "order" | "production") {
    if (!day) return;
    const items = type === "order" ? orderItems : productionItems;
    if (total(items) === 0) {
      setError("数量を選んでください");
      return;
    }
    act({ type, day, items });
  }
  function updateOrder(
    order: Order,
    type:
      | "complete"
      | "cancel"
      | "skip"
      | "resume"
      | "prioritize"
      | "unprioritize",
  ) {
    if (!day) return;
    if (
      type === "cancel" &&
      !window.confirm(`${order.number}番の注文をキャンセルしますか？`)
    )
      return;
    act({ type, day, orderId: order.id });
    if (type === "prioritize" || type === "unprioritize")
      document.querySelector<HTMLInputElement>("#ticket-search")?.focus();
  }
  function receive() {
    if (disabled || !selectedOrder) return;
    if (selectedOrder.status === "skipped") {
      updateOrder(selectedOrder, "resume");
      document.querySelector<HTMLInputElement>("#ticket-search")?.focus();
      return;
    }
    if (
      selectedOrder.status !== "ready" ||
      !window.confirm(`${selectedOrder.number}番を受け渡し済みにしますか？`)
    )
      return;
    updateOrder(selectedOrder, "complete");
    setSearch("");
    document.querySelector<HTMLInputElement>("#ticket-search")?.focus();
  }
  function skipSelected() {
    if (disabled || !selectedOrder || !canSkip) return;
    updateOrder(selectedOrder, "skip");
    setSearch("");
    document.querySelector<HTMLInputElement>("#ticket-search")?.focus();
  }
  function renderEntry(type: "order" | "production") {
    if (!state) return null;
    return (
      <section
        className="entry-panel"
        id={`entry-${type}`}
        key={type}
        aria-labelledby={`heading-${type}`}
      >
        <div className="panel-heading">
          <h1 id={`heading-${type}`}>
            {type === "order" ? "注文受付" : "焼き上がり"}
          </h1>
        </div>
        <QuantityPicker
          key={formVersion[type]}
          values={type === "order" ? orderItems : productionItems}
          onChange={type === "order" ? setOrderItems : setProductionItems}
          disabled={disabled}
          unavailable={
            type === "order"
              ? state.flavors
                  .filter(({ enabled }) => !enabled)
                  .map(({ id }) => id)
              : []
          }
          production={type === "production"}
          onSubmit={() => submit(type)}
        />
        <div className="entry-total">
          <span>合計</span>
          <strong>
            {total(type === "order" ? orderItems : productionItems)}
            <small>個</small>
          </strong>
          {type === "production" && total(productionItems) > 20 ? (
            <span>20個を超えています</span>
          ) : null}
        </div>
        <button
          className="primary submit"
          disabled={
            disabled ||
            total(type === "order" ? orderItems : productionItems) === 0
          }
          onClick={() => submit(type)}
          aria-keyshortcuts={type === "order" ? "Enter" : "F"}
        >
          {type === "order" ? "注文を登録" : "焼き上がりを登録"}
          <span>
            {type === "order" ? `${state.day.lastNumber + 1}番 ` : ""}
            <kbd>{type === "order" ? "↵" : "F"}</kbd>
          </span>
        </button>
      </section>
    );
  }
  if (!token)
    return (
      <main className="login-shell">
        <a className="brand" href="/">
          taiyaq<span>たい焼き</span>
        </a>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            sessionStorage.setItem("taiyaq:staff", login);
            setToken(login);
          }}
        >
          <h1>スタッフログイン</h1>
          <label htmlFor="staff-token">スタッフ用トークン</label>
          <input
            id="staff-token"
            type="password"
            autoComplete="current-password"
            value={login}
            onChange={(event) => setLogin(event.target.value)}
            required
          />
          <button className="primary">ログイン</button>
        </form>
      </main>
    );
  return (
    <>
      <header className="app-header">
        <a className="brand" href="/staff">
          taiyaq<span>スタッフ</span>
        </a>
        <nav aria-label="操作画面">
          {views.map(({ id, label, key }) => (
            <button
              key={id}
              aria-current={view === id ? "page" : undefined}
              onClick={() => setView(id)}
              aria-keyshortcuts={key}
            >
              {label}
              <kbd>{key}</kbd>
            </button>
          ))}
        </nav>
        <div className="header-actions">
          {state ? (
            <div className="ready-total">
              販売済み <strong>{state.soldCount}</strong>個
            </div>
          ) : null}
          <button
            className="text-button"
            disabled={disabled}
            onClick={() => {
              sessionStorage.removeItem("taiyaq:staff");
              setToken("");
            }}
          >
            ログアウト
          </button>
        </div>
      </header>
      <div className="service-bar">
        <span>{day ?? "読み込み中…"}</span>
        <Freshness
          error={poll.error}
          updatedAt={poll.updatedAt}
          live={poll.live}
        />
        <a href="/display" target="_blank" rel="noreferrer">
          店頭モニター ↗
        </a>
      </div>
      {notice ? (
        <div className="notice" role="status">
          {notice}
        </div>
      ) : null}
      {error || poll.error ? (
        <div className="error-banner" role="alert">
          {error || poll.error}
        </div>
      ) : null}
      {pending ? (
        <div className="pending-banner" role="status">
          {busy ? "送信中…" : "未確認の操作があります"}
          <button
            disabled={busy}
            onClick={() => {
              void transmit(pending);
            }}
          >
            同じ操作を再送信
          </button>
        </div>
      ) : null}
      {!state ? (
        <main className="empty-state">
          <h1>{poll.error ? "接続を確認してください" : "読み込み中…"}</h1>
          <button onClick={poll.refresh}>再読み込み</button>
        </main>
      ) : (
        <main
          className={`staff-workspace ${view === "order" ? "with-production" : ""}`}
        >
          {view === "settings" || view === "history" ? (
            <section className="entry-panel">
              {view === "settings" ? (
                <Settings state={state} disabled={disabled} act={act} />
              ) : (
                <History token={token} state={state} live={poll.live} />
              )}
            </section>
          ) : (
            renderEntry(view)
          )}
          <section className="orders-panel">
            <div className="panel-heading">
              <h2>
                注文一覧 <span>{state.orders.length}</span>
              </h2>
              <div className="ticket-search">
                <label className="sr-only" htmlFor="ticket-search">
                  整理券番号で検索
                </label>
                <input
                  id="ticket-search"
                  type="text"
                  inputMode="numeric"
                  pattern="[0-9]*"
                  placeholder="整理券番号 /"
                  value={search}
                  onChange={(event) =>
                    setSearch(event.target.value.replace(/\D/g, ""))
                  }
                  onKeyDown={(event) => {
                    if (event.key === "Escape") {
                      setSearch("");
                      event.currentTarget.blur();
                      return;
                    }
                    if (
                      event.repeat ||
                      event.nativeEvent.isComposing ||
                      event.ctrlKey ||
                      event.metaKey ||
                      event.altKey
                    )
                      return;
                    if (event.key === "Enter") {
                      event.preventDefault();
                      receive();
                    }
                    if (event.key === ".") {
                      event.preventDefault();
                      skipSelected();
                    }
                  }}
                />
                <button
                  className="primary"
                  disabled={
                    disabled ||
                    (selectedOrder?.status !== "ready" &&
                      selectedOrder?.status !== "skipped")
                  }
                  onClick={receive}
                >
                  {selectedOrder?.status === "skipped"
                    ? "列に戻す"
                    : "受け渡し"}{" "}
                  <kbd>↵</kbd>
                </button>
                <button
                  disabled={disabled || !canSkip}
                  onClick={skipSelected}
                  aria-keyshortcuts="."
                >
                  飛ばす <kbd>.</kbd>
                </button>
              </div>
            </div>
            <div className="stock-strip">
              {FLAVORS.map(({ id, name }) => (
                <div key={id}>
                  <span>{name}</span>
                  <strong>
                    {state.orders.reduce(
                      (sum, order) => sum + order.allocated[id],
                      state.stock[id],
                    )}
                  </strong>
                  <small className="stock-surplus">
                    余剰 {state.stock[id]}
                  </small>
                </div>
              ))}
            </div>
            <div className="order-list">
              {state.orders
                .filter(({ number }) => !search || number === Number(search))
                .toSorted(
                  (a, b) =>
                    Number(a.status === "skipped") -
                      Number(b.status === "skipped") ||
                    Number(b.status === "ready") -
                      Number(a.status === "ready") ||
                    compareAllocationOrder(a, b),
                )
                .map((order) => {
                  const waitMinutes = estimateOrderWait(state, order);
                  return (
                    <article
                      className={`order-row ${order.status}${order.prioritySequence === null ? "" : " priority"}${recentReady.has(order.id) ? " recent-ready" : ""}`}
                      key={order.id}
                    >
                      <strong className="ticket-number">
                        {order.number.toString().padStart(2, "0")}
                      </strong>
                      <div className="order-content">
                        <span className={`status-label ${order.status}`}>
                          {order.prioritySequence === null
                            ? STATUS_LABELS[order.status]
                            : "優先待ち"}
                        </span>
                        {order.status === "waiting" ? (
                          <p className="order-wait">
                            {waitMinutes === 0
                              ? "そろそろできます"
                              : `約${waitMinutes}分後`}
                          </p>
                        ) : null}
                        <div className="order-items">
                          {FLAVORS.filter(({ id }) => order.items[id] > 0).map(
                            ({ id, name }) => (
                              <span key={id}>
                                {name} <b>{order.items[id]}</b>
                                {order.status === "waiting" ? (
                                  <small>確保 {order.allocated[id]}</small>
                                ) : null}
                              </span>
                            ),
                          )}
                        </div>
                      </div>
                      <div className="order-actions">
                        {order.status === "waiting" ? (
                          <button
                            disabled={disabled}
                            aria-pressed={order.prioritySequence !== null}
                            onClick={() =>
                              updateOrder(
                                order,
                                order.prioritySequence === null
                                  ? "prioritize"
                                  : "unprioritize",
                              )
                            }
                          >
                            {order.prioritySequence === null
                              ? "優先する"
                              : "優先を解除"}
                          </button>
                        ) : null}
                        {order.status === "ready" ? (
                          <button
                            className="primary"
                            disabled={disabled}
                            onClick={() => updateOrder(order, "complete")}
                          >
                            受け渡し
                          </button>
                        ) : null}
                        <button
                          className={
                            order.status === "skipped" ? "primary" : ""
                          }
                          disabled={disabled}
                          onClick={() =>
                            updateOrder(
                              order,
                              order.status === "skipped" ? "resume" : "skip",
                            )
                          }
                        >
                          {order.status === "skipped" ? "列に戻す" : "飛ばす"}
                        </button>
                        <button
                          className="text-button"
                          disabled={disabled}
                          onClick={() => updateOrder(order, "cancel")}
                        >
                          取消
                        </button>
                      </div>
                    </article>
                  );
                })}
              {!state.orders.length ? (
                <div className="empty-state">未受け取りの注文はありません</div>
              ) : null}
            </div>
          </section>
          {view === "order" ? renderEntry("production") : null}
        </main>
      )}
    </>
  );
}

export function Freshness({
  error,
  updatedAt,
  live,
}: {
  error?: string;
  updatedAt?: number;
  live?: boolean;
}) {
  return (
    <span className={`freshness ${error ? "offline" : ""}`}>
      {error
        ? "更新できていません"
        : live
          ? "リアルタイム更新中"
          : updatedAt
            ? `${live === false ? "定期更新中 · " : "更新 "}${new Date(updatedAt).toLocaleTimeString("ja-JP", { hour12: false, timeZone: "Asia/Tokyo" })}`
            : "接続中…"}
    </span>
  );
}

function Settings({
  state,
  disabled,
  act,
}: {
  state: State;
  disabled: boolean;
  act: (command: Command) => void;
}) {
  return (
    <>
      <div className="panel-heading">
        <h1>販売設定</h1>
      </div>
      <div className="flavor-settings">
        {state.flavors.map((flavor) => (
          <FlavorSettings
            key={`${flavor.id}-${flavor.enabled}-${flavor.cookingMinutes}-${flavor.perBatch}`}
            flavor={flavor}
            disabled={disabled}
            act={act}
          />
        ))}
      </div>
      <form
        className="adjustment"
        onSubmit={(event) => {
          event.preventDefault();
          if (
            window.confirm(
              "在庫を訂正しますか？ 準備中の注文への割り当ても再計算します。",
            )
          ) {
            const fields = new FormData(event.currentTarget);
            const items = zeroCounts();
            for (const { id } of FLAVORS) items[id] = Number(fields.get(id));
            act({ type: "adjust", day: state.day.id, items });
          }
        }}
      >
        <h2>在庫の訂正</h2>
        <p>増やす数は正、減らす数は負で入力</p>
        {FLAVORS.map(({ id, name }) => (
          <label key={id}>
            {name}
            <input
              type="number"
              min={-100}
              max={100}
              step={1}
              name={id}
              defaultValue={0}
              disabled={disabled}
              required
            />
          </label>
        ))}
        <button disabled={disabled}>在庫を訂正</button>
      </form>
    </>
  );
}

function FlavorSettings({
  flavor,
  disabled,
  act,
}: {
  flavor: Flavor;
  disabled: boolean;
  act: (command: Command) => void;
}) {
  const [minutes, setMinutes] = useState(flavor.cookingMinutes);
  const [batch, setBatch] = useState(flavor.perBatch);
  return (
    <form
      className="flavor-setting"
      onSubmit={(event) => {
        event.preventDefault();
        act({
          type: "flavor",
          flavor: { ...flavor, cookingMinutes: minutes, perBatch: batch },
        });
      }}
    >
      <div>
        <strong>{FLAVORS.find(({ id }) => id === flavor.id)!.name}</strong>
        <button
          type="button"
          className={`availability ${flavor.enabled ? "on" : ""}`}
          aria-pressed={flavor.enabled}
          disabled={disabled}
          onClick={() =>
            act({
              type: "flavor",
              flavor: { ...flavor, enabled: !flavor.enabled },
            })
          }
        >
          {flavor.enabled ? "受付中" : "受付停止"}
        </button>
      </div>
      <div className="setting-fields">
        <label>
          焼き時間（分）
          <input
            type="number"
            min={1}
            max={120}
            value={minutes}
            onChange={(event) => setMinutes(Number(event.target.value))}
            required
          />
        </label>
        <label>
          1回の目安（個）
          <input
            type="number"
            min={1}
            max={20}
            value={batch}
            onChange={(event) => setBatch(Number(event.target.value))}
            required
          />
        </label>
        <button
          disabled={
            disabled ||
            (minutes === flavor.cookingMinutes && batch === flavor.perBatch)
          }
        >
          保存
        </button>
      </div>
    </form>
  );
}

function History({
  token,
  state,
  live,
}: {
  token: string;
  state: State;
  live: boolean;
}) {
  const options = {
    refreshKey: `${state.day.id}:${state.revision}`,
    interval: live ? (false as const) : 2000,
  };
  const days = usePolling<Day[]>("/api/staff/days", token, options);
  const [day, setDay] = useState("");
  const selectedDay = day || days.data?.[0]?.id;
  const history = usePolling<Order[]>(
    selectedDay ? `/api/staff/history/${selectedDay}` : null,
    token,
    options,
  );
  return (
    <>
      <div className="panel-heading">
        <h1>履歴</h1>
      </div>
      <div className="day-list">
        {days.data?.map(({ id }) => (
          <button
            key={id}
            aria-pressed={id === selectedDay}
            onClick={() => setDay(id)}
          >
            {id}
          </button>
        ))}
      </div>
      {days.error || history.error ? (
        <p role="alert">{days.error || history.error}</p>
      ) : null}
      {history.data?.map((order) => (
        <article className="history-row" key={order.id}>
          <strong>{order.number}番</strong>
          <span>{STATUS_LABELS[order.status]}</span>
          <p>
            {FLAVORS.filter(({ id }) => order.items[id] > 0)
              .map(({ id, name }) => `${name} ${order.items[id]}個`)
              .join("、")}
          </p>
        </article>
      ))}
    </>
  );
}
