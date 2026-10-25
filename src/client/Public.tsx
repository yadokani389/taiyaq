import { useEffect, useState } from "react";
import {
  FLAVORS,
  STATUS_LABELS,
  type PublicOrder,
  type PublicState,
} from "../shared";
import { usePolling } from "./api";
import { Freshness } from "./Staff";

export function Public({ display = false }: { display?: boolean }) {
  useEffect(() => {
    if (!display) return;
    const timer = setInterval(() => {
      for (const grid of document.querySelectorAll<HTMLElement>(
        ".display-page .number-grid",
      )) {
        if (grid.scrollHeight <= grid.clientHeight) continue;
        const top =
          grid.scrollTop + grid.clientHeight >= grid.scrollHeight - 2
            ? 0
            : grid.scrollTop + grid.clientHeight;
        grid.scrollTo({
          top,
          behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
            ? "instant"
            : "smooth",
        });
      }
    }, 8000);
    return () => clearInterval(timer);
  }, [display]);
  const status = usePolling<PublicState>("/api/status");
  const [number, setNumber] = useState("");
  const [searchError, setSearchError] = useState("");
  const [selected, setSelected] = useState<string | null>(
    () =>
      new URLSearchParams(location.search).get("order") ??
      localStorage.getItem("taiyaq:order"),
  );
  const detail = usePolling<{ order: PublicOrder; waitMinutes: number }>(
    !display && selected ? `/api/orders/${encodeURIComponent(selected)}` : null,
  );
  function selectOrder(id: string) {
    setSearchError("");
    setSelected(id);
    localStorage.setItem("taiyaq:order", id);
    const url = new URL(location.href);
    url.searchParams.set("order", id);
    history.replaceState(null, "", url);
  }
  const data = status.data;
  const ready = data?.orders.filter(({ status }) => status === "ready") ?? [];
  const waiting =
    data?.orders.filter(({ status }) => status === "waiting") ?? [];
  return (
    <div className={display ? "display-page" : "public-page"}>
      <header className="public-header">
        <a className="brand" href="/">
          taiyaq<span>たい焼き</span>
        </a>
        <Freshness error={status.error} updatedAt={status.updatedAt} />
      </header>
      {status.error ? (
        <div className="error-banner" role="alert">
          通信を確認してください。表示は最後に取得した情報です。
        </div>
      ) : null}
      {!data ? (
        <main className="empty-state">
          <h1>読み込み中…</h1>
        </main>
      ) : !data.day ? (
        <main className="empty-state">
          <h1>ただいま営業時間外です</h1>
          {detail.data ? (
            <p>
              {detail.data.order.day}　{detail.data.order.number}番：
              {STATUS_LABELS[detail.data.order.status]}
            </p>
          ) : null}
        </main>
      ) : (
        <main>
          {!display ? (
            <section className="your-order">
              <form
                onSubmit={(event) => {
                  event.preventDefault();
                  const order = data.orders.find(
                    (order) => order.number === Number(number),
                  );
                  if (order) selectOrder(order.id);
                  else
                    setSearchError("この番号の未受け取り注文は見つかりません");
                }}
              >
                <label htmlFor="your-number">整理券番号で確認</label>
                <div>
                  <input
                    id="your-number"
                    type="number"
                    inputMode="numeric"
                    min={1}
                    max={100}
                    placeholder="1〜100"
                    value={number}
                    onChange={(event) => setNumber(event.target.value)}
                    required
                  />
                  <button className="primary">確認</button>
                </div>
              </form>
              {searchError || detail.error ? (
                <p role="alert">{searchError || detail.error}</p>
              ) : null}
              {detail.data ? (
                <article
                  className={`your-order-detail ${detail.data.order.status}`}
                  aria-live="polite"
                >
                  <strong>
                    {detail.data.order.number}
                    <small>番</small>
                  </strong>
                  <div>
                    <h1>{STATUS_LABELS[detail.data.order.status]}</h1>
                    {detail.data.order.status === "waiting" ? (
                      <p>待ち時間の目安：約{detail.data.waitMinutes}分</p>
                    ) : null}
                    {detail.data.order.status === "ready" ? (
                      <p>整理券を持って受け取り口へお越しください</p>
                    ) : null}
                    <p>
                      {FLAVORS.filter(
                        ({ id }) => detail.data!.order.items[id] > 0,
                      )
                        .map(
                          ({ id, name }) =>
                            `${name} ${detail.data!.order.items[id]}個`,
                        )
                        .join("、")}
                    </p>
                    {detail.data.order.day !== data.day ? (
                      <small>{detail.data.order.day}の注文</small>
                    ) : null}
                  </div>
                </article>
              ) : null}
            </section>
          ) : null}
          <section className="number-section ready-numbers">
            <div className="number-heading">
              <h1>お受け取りいただけます</h1>
              <span>{ready.length}件</span>
            </div>
            <div className="number-grid" aria-live="polite">
              {ready.map(({ id, number }) =>
                display ? (
                  <span key={id}>{number.toString().padStart(2, "0")}</span>
                ) : (
                  <button
                    key={id}
                    onClick={() => selectOrder(id)}
                    aria-label={`${number}番 受け取り可能`}
                  >
                    {number.toString().padStart(2, "0")}
                  </button>
                ),
              )}
              {!ready.length ? (
                <p className="number-empty">できあがり次第、番号を表示します</p>
              ) : null}
            </div>
          </section>
          <section className="number-section waiting-numbers">
            <div className="number-heading">
              <h2>準備中</h2>
              <span>{waiting.length}件</span>
            </div>
            <div className="number-grid">
              {waiting.map(({ id, number }) =>
                display ? (
                  <span key={id}>{number.toString().padStart(2, "0")}</span>
                ) : (
                  <button
                    key={id}
                    onClick={() => selectOrder(id)}
                    aria-label={`${number}番 準備中`}
                  >
                    {number.toString().padStart(2, "0")}
                  </button>
                ),
              )}
              {!waiting.length ? (
                <p className="number-empty">お待ちの注文はありません</p>
              ) : null}
            </div>
          </section>
          {!display ? (
            <section className="menu-section">
              <h2>現在のメニュー</h2>
              <div>
                {data.flavors.map((flavor) => (
                  <article
                    key={flavor.id}
                    className={flavor.enabled ? "" : "unavailable"}
                  >
                    <span className={`flavor-mark flavor-${flavor.id}`} />
                    <h3>{FLAVORS.find(({ id }) => id === flavor.id)!.name}</h3>
                    <p>
                      {flavor.enabled
                        ? `目安 約${flavor.waitMinutes}分`
                        : "受付停止"}
                    </p>
                  </article>
                ))}
              </div>
            </section>
          ) : (
            <p className="display-instruction">
              整理券を持って受け取り口へお越しください
            </p>
          )}
        </main>
      )}
      {!display ? (
        <footer className="public-footer">
          店頭でご注文後、整理券番号でできあがりを確認できます。
        </footer>
      ) : null}
    </div>
  );
}
