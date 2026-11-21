import { readFile, writeFile } from "node:fs/promises";

const directory = new URL("../dist/demo/", import.meta.url);
const index = new URL("index.html", directory);
const html = (await readFile(index, "utf8")).replace(
  "taiyaq — たい焼き注文状況",
  "taiyaq | 文化祭のたい焼き販売",
);
const pages = {
  index: "",
  staff: " | スタッフ操作デモ",
  display: " | 店頭モニターデモ",
  order: " | 来場者向けデモ",
  404: " | ページが見つかりません",
};
await Promise.all([
  ...Object.entries(pages).map(([file, title]) =>
    writeFile(
      new URL(`${file}.html`, directory),
      html.replace("</title>", `${title}</title>`),
    ),
  ),
  writeFile(
    new URL("_headers", directory),
    `/*\n  Content-Security-Policy: connect-src 'none'\n  X-Content-Type-Options: nosniff\n`,
  ),
]);
