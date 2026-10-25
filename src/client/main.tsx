import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Staff } from "./Staff";
import { Public } from "./Public";
import "./style.css";

const path = location.pathname.replace(/\/$/, "") || "/";
const page =
  path === "/staff" ? (
    <Staff />
  ) : path === "/display" ? (
    <Public display />
  ) : path === "/" ? (
    <Public />
  ) : (
    <main className="empty-state">
      <h1>ページが見つかりません</h1>
      <a href="/">注文状況を確認</a>
    </main>
  );
createRoot(document.getElementById("root")!).render(
  <StrictMode>{page}</StrictMode>,
);
