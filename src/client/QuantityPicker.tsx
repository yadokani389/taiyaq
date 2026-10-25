import { useEffect, useState } from "react";
import { FLAVORS, type Counts, type FlavorId } from "../shared";

type Props = {
  values: Counts;
  onChange: (values: Counts) => void;
  disabled: boolean;
  unavailable?: FlavorId[];
  production?: boolean;
  onSubmit: () => void;
};

export function QuantityPicker({
  values,
  onChange,
  disabled,
  unavailable = [],
  production = false,
  onSubmit,
}: Props) {
  const available = FLAVORS.filter(({ id }) => !unavailable.includes(id));
  const [selected, setSelected] = useState<FlavorId>("tsubuan");
  const activeId = available.some(({ id }) => id === selected)
    ? selected
    : available[0]?.id;
  function setQuantity(id: FlavorId, number: number) {
    setSelected(id);
    onChange({ ...values, [id]: Math.max(0, Math.min(100, number)) });
  }
  useEffect(() => {
    function handleKey(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null;
      if (
        disabled ||
        !activeId ||
        event.isComposing ||
        target?.closest(
          'input, textarea, select, [contenteditable="true"], dialog',
        )
      )
        return;
      if (event.ctrlKey || event.metaKey || event.altKey || event.repeat)
        return;
      if (event.key === "Enter" && !target?.closest("button, a")) {
        event.preventDefault();
        onSubmit();
        return;
      }
      if (!/^(\d|ArrowDown|ArrowUp|ArrowRight|ArrowLeft)$/.test(event.key))
        return;
      event.preventDefault();
      target?.blur();
      if (/^\d$/.test(event.key)) {
        setQuantity(activeId, Number(event.key));
      }
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        const index = available.findIndex(({ id }) => id === activeId);
        setSelected(
          available[
            (index + (event.key === "ArrowDown" ? 1 : available.length - 1)) %
              available.length
          ]!.id,
        );
      }
      if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
        setQuantity(
          activeId,
          values[activeId] + (event.key === "ArrowRight" ? 1 : -1),
        );
      }
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  });
  const presets = production ? [0, 1, 2, 5, 10, 20] : [0, 1, 2, 3];
  return (
    <div className={`quantity-picker ${production ? "production" : ""}`}>
      {FLAVORS.map(({ id, name }, index) => (
        <fieldset
          className={`quantity-row ${activeId === id ? "selected" : ""}`}
          key={id}
          disabled={disabled}
        >
          <legend>
            <span className={`flavor-mark flavor-${id}`} />
            {name}
            {unavailable.includes(id) ? <small>受付停止</small> : null}
          </legend>
          <div className="quantity-controls">
            <output aria-label={`${name}の数量`}>{values[id]}</output>
            <div className="presets">
              {presets.map((n) => (
                <button
                  type="button"
                  key={n}
                  disabled={unavailable.includes(id) && n !== 0}
                  aria-label={`${name} ${n}個`}
                  aria-pressed={values[id] === n}
                  onClick={() => setQuantity(id, n)}
                >
                  {n}
                </button>
              ))}
            </div>
            <button
              type="button"
              className="step"
              aria-label={`${name}を1個減らす`}
              disabled={unavailable.includes(id)}
              onClick={() => setQuantity(id, values[id] - 1)}
            >
              −
            </button>
            <button
              type="button"
              className="step"
              aria-label={`${name}を1個増やす`}
              disabled={unavailable.includes(id)}
              onClick={() => setQuantity(id, values[id] + 1)}
            >
              ＋
            </button>
          </div>
          <span className="sr-only">味 {index + 1}</span>
        </fieldset>
      ))}
      <p className="key-guide">
        <kbd>↑</kbd>
        <kbd>↓</kbd> 味　<kbd>0–9</kbd> 数量　<kbd>←</kbd>
        <kbd>→</kbd> 増減
      </p>
    </div>
  );
}
