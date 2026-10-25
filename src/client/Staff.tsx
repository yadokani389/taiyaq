import { useEffect, useState } from "react";
import * as v from "valibot";
import {
  FLAVORS,
  MAX_TICKET_NUMBER,
  STATUS_LABELS,
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

type View = "order" | "production" | "settings" | "history";
const views: Array<{ id: View; label: string; key: string }> = [
  { id: "order", label: "注文受付", key: "O" },
  { id: "production", label: "焼き上がり", key: "B" },
  { id: "settings", label: "販売設定", key: "S" },
  { id: "history", label: "履歴", key: "H" },
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
  const [formVersion, setFormVersion] = useState(0);
  const [pending, setPending] = useState<CommandRequest | undefined>(
    storedRequest,
  );
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");
  const poll = usePolling<State>(token ? "/api/staff/state" : null, token);
  const state = poll.data;
  const day = state?.day.id;
  const disabled = busy || Boolean(pending);
  const receiving = state?.orders.find(
    ({ number, status }) => number === Number(search) && status === "ready",
  );

  useEffect(() => {
    function navigate(event: KeyboardEvent) {
      if (
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
        setFormVersion((version) => version + 1);
        setNotice(`${result.order!.number}番で受け付けました`);
      } else if (request.command.type === "production") {
        setProductionItems(zeroCounts());
        setFormVersion((version) => version + 1);
        setNotice("焼き上がりを登録しました");
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
  function submit() {
    if (!day) return;
    const items = view === "order" ? orderItems : productionItems;
    if (total(items) === 0) {
      setError("数量を選んでください");
      return;
    }
    act({ type: view === "order" ? "order" : "production", day, items });
  }
  function finish(order: Order, type: "complete" | "cancel") {
    if (!day) return;
    if (
      type === "cancel" &&
      !window.confirm(`${order.number}番の注文をキャンセルしますか？`)
    )
      return;
    act({ type, day, orderId: order.id });
  }
  function receive() {
    if (
      disabled ||
      !receiving ||
      !window.confirm(`${receiving.number}番を受け渡し済みにしますか？`)
    )
      return;
    finish(receiving, "complete");
    setSearch("");
    document.querySelector<HTMLInputElement>("#ticket-search")?.focus();
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
            >
              {label}
              <kbd>{key}</kbd>
            </button>
          ))}
        </nav>
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
      </header>
      <div className="service-bar">
        <span>{day ?? "読み込み中…"}</span>
        <Freshness error={poll.error} updatedAt={poll.updatedAt} />
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
        <main className="staff-workspace">
          <section className="entry-panel">
            {view === "settings" ? (
              <Settings state={state} disabled={disabled} act={act} />
            ) : view === "history" ? (
              <History token={token} />
            ) : (
              <>
                <div className="panel-heading">
                  <h1>{view === "order" ? "注文受付" : "焼き上がり"}</h1>
                </div>
                <QuantityPicker
                  key={`${view}-${formVersion}`}
                  values={view === "order" ? orderItems : productionItems}
                  onChange={
                    view === "order" ? setOrderItems : setProductionItems
                  }
                  disabled={disabled}
                  unavailable={
                    view === "order"
                      ? state.flavors
                          .filter(({ enabled }) => !enabled)
                          .map(({ id }) => id)
                      : []
                  }
                  production={view === "production"}
                  onSubmit={submit}
                />
                <div className="entry-total">
                  <span>合計</span>
                  <strong>
                    {total(view === "order" ? orderItems : productionItems)}
                    <small>個</small>
                  </strong>
                  {view === "production" && total(productionItems) > 20 ? (
                    <span>20個を超えています</span>
                  ) : null}
                </div>
                <button
                  className="primary submit"
                  disabled={
                    disabled ||
                    total(view === "order" ? orderItems : productionItems) === 0
                  }
                  onClick={submit}
                >
                  {view === "order" ? "注文を登録" : "焼き上がりを登録"}
                  <kbd>↵</kbd>
                </button>
              </>
            )}
          </section>
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
                    setSearch(
                      event.target.value
                        .replace(/\D/g, "")
                        .slice(0, String(MAX_TICKET_NUMBER).length),
                    )
                  }
                  onKeyDown={(event) => {
                    if (event.key === "Escape") {
                      setSearch("");
                      event.currentTarget.blur();
                    }
                    if (
                      event.key === "Enter" &&
                      !event.repeat &&
                      !event.nativeEvent.isComposing &&
                      !event.ctrlKey &&
                      !event.metaKey &&
                      !event.altKey
                    ) {
                      event.preventDefault();
                      receive();
                    }
                  }}
                />
                <button
                  className="primary"
                  disabled={disabled || !receiving}
                  onClick={receive}
                >
                  受け渡し <kbd>↵</kbd>
                </button>
              </div>
            </div>
            <div className="stock-strip">
              {FLAVORS.map(({ id, name }) => (
                <div key={id}>
                  <span>{name}</span>
                  <strong>{state.stock[id]}</strong>
                  <small>余剰</small>
                </div>
              ))}
            </div>
            <div className="order-list">
              {state.orders
                .filter(({ number }) => !search || number === Number(search))
                .toSorted(
                  (a, b) =>
                    Number(b.status === "ready") -
                      Number(a.status === "ready") || a.sequence - b.sequence,
                )
                .map((order) => (
                  <article
                    className={`order-row ${order.status}`}
                    key={order.id}
                  >
                    <strong className="ticket-number">
                      {order.number.toString().padStart(2, "0")}
                    </strong>
                    <div className="order-content">
                      <span className={`status-label ${order.status}`}>
                        {STATUS_LABELS[order.status]}
                      </span>
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
                      {order.status === "ready" ? (
                        <button
                          className="primary"
                          disabled={disabled}
                          onClick={() => finish(order, "complete")}
                        >
                          受け渡し
                        </button>
                      ) : null}
                      <button
                        className="text-button"
                        disabled={disabled}
                        onClick={() => finish(order, "cancel")}
                      >
                        取消
                      </button>
                    </div>
                  </article>
                ))}
              {!state.orders.length ? (
                <div className="empty-state">未受け取りの注文はありません</div>
              ) : null}
            </div>
          </section>
        </main>
      )}
    </>
  );
}

export function Freshness({
  error,
  updatedAt,
}: {
  error?: string;
  updatedAt?: number;
}) {
  return (
    <span className={`freshness ${error ? "offline" : ""}`}>
      {error
        ? "更新できていません"
        : updatedAt
          ? `更新 ${new Date(updatedAt).toLocaleTimeString("ja-JP", { hour12: false, timeZone: "Asia/Tokyo" })}`
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
              "在庫を訂正しますか？ 未受け取りの注文への割り当ても再計算します。",
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

function History({ token }: { token: string }) {
  const days = usePolling<Day[]>("/api/staff/days", token);
  const [day, setDay] = useState("");
  const selectedDay = day || days.data?.[0]?.id;
  const history = usePolling<Order[]>(
    selectedDay ? `/api/staff/history/${selectedDay}` : null,
    token,
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
