import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Staff } from "./Staff";
import { Public } from "./Public";
import "./style.css";

const demo = import.meta.env.MODE === "demo";

function Home() {
  return (
    <main className="festival-home">
      <section className="festival-intro">
        <p className="festival-brand">taiyaq</p>
        <h1>
          <span>たい焼き販売に</span>
          <span>お越しいただき、</span>
          <span>ありがとうございました</span>
        </h1>
        <p>
          文化祭では、たい焼きを合計<strong>841個</strong>販売しました。
        </p>
        <p>文化祭で使った注文・受け取りシステムをお試しいただけます。</p>
      </section>
      <nav className="festival-demos" aria-label="デモ画面">
        <a href="/staff">
          スタッフ操作デモ{" "}
          <span>
            注文を受け付け、焼き上がりから受け渡しまで操作する（PC推奨）
          </span>
        </a>
        <a href="/display">
          店頭モニターデモ <span>受け取り可能な整理券番号を見る</span>
        </a>
        <a href="/order">
          来場者向けデモ <span>整理券番号で注文状況を確かめる</span>
        </a>
      </nav>
      <p className="festival-note">
        注文はすべて架空です。同じブラウザの別タブを開くと、スタッフ操作が各画面に反映されます。
      </p>
    </main>
  );
}

const path = location.pathname.replace(/\/$/, "") || "/";
const page =
  demo && path === "/" ? (
    <Home />
  ) : path === "/staff" ? (
    <Staff />
  ) : path === "/display" ? (
    <Public display />
  ) : path === (demo ? "/order" : "/") ? (
    <Public />
  ) : (
    <main className="empty-state">
      <h1>ページが見つかりません</h1>
      <a href="/">{demo ? "トップに戻る" : "注文状況を確認"}</a>
    </main>
  );
createRoot(document.getElementById("root")!).render(
  <StrictMode>{page}</StrictMode>,
);
