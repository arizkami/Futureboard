import { useRef, useEffect, type ReactNode } from "react";
import {
  clamp,
  normalizeUltraVerbParams,
  serializeUltraVerbParams,
  type UltraVerbMode,
  type UltraVerbParams,
} from "../Core";

type Props = {
  params: Record<string, number | string | boolean>;
  enabled: boolean;
  onParamsChange: (patch: Record<string, number | string | boolean>) => void;
  onToggleEnabled: () => void;
  onReset: () => void;
};

const MODES: { id: UltraVerbMode; label: string }[] = [
  { id: "room",  label: "Room"  },
  { id: "plate", label: "Plate" },
  { id: "hall",  label: "Hall"  },
  { id: "space", label: "Space" },
];

// ── Canvas tail renderer ─────────────────────────────────────────────────────

function paintTail(canvas: HTMLCanvasElement, m: UltraVerbParams) {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const dpr = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  if (!rect.width || !rect.height) return;

  canvas.width  = Math.round(rect.width  * dpr);
  canvas.height = Math.round(rect.height * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

  const w = rect.width;
  const h = rect.height;
  ctx.clearRect(0, 0, w, h);

  const decayN   = clamp((m.decay - 0.1) / 19.9, 0, 1);
  const dampN    = m.damping / 100;
  const widthN   = clamp(m.width / 150, 0, 1);
  const mixN     = m.mix / 100;
  const density  = m.diffusion / 100;

  // Tail waveform stroke
  const strokeG = ctx.createLinearGradient(0, 0, w, 0);
  strokeG.addColorStop(0,    `rgba(124,199,255,${(0.22 + mixN * 0.18).toFixed(2)})`);
  strokeG.addColorStop(0.52, `rgba(167,139,250,${(0.18 + widthN * 0.18).toFixed(2)})`);
  strokeG.addColorStop(1,    "rgba(124,199,255,0.02)");

  ctx.lineWidth   = 2;
  ctx.strokeStyle = strokeG;
  ctx.beginPath();
  for (let i = 0; i <= 120; i++) {
    const t   = i / 120;
    const x   = 18 + t * (w - 36);
    const env = Math.exp(-t * (2.1 + (1 - decayN) * 3.2));
    const y   = h * 0.68 - env * h * (0.42 + mixN * 0.08)
              + Math.sin(t * Math.PI * (5 + density * 8)) * (12 + widthN * 10) * env;
    if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
  }
  ctx.stroke();

  // Fill under tail
  const fillG = ctx.createLinearGradient(0, h * 0.2, 0, h);
  fillG.addColorStop(0,   "rgba(124,199,255,0.18)");
  fillG.addColorStop(0.7, "rgba(167,139,250,0.04)");
  fillG.addColorStop(1,   "rgba(124,199,255,0)");
  ctx.lineTo(w - 18, h - 18);
  ctx.lineTo(18, h - 18);
  ctx.closePath();
  ctx.fillStyle = fillG;
  ctx.fill();

  // Early reflection bars
  for (let i = 0; i < 18; i++) {
    const t = i / 17;
    ctx.fillStyle = `rgba(237,242,255,${((1 - t) * (0.4 + density * 0.36)).toFixed(2)})`;
    ctx.fillRect(
      24 + t * w * 0.34,
      28 + ((i * 31) % Math.max(60, h - 56)),
      2,
      18 * (1 - t * 0.72),
    );
  }

  // Horizontal grid lines
  ctx.lineWidth = 1;
  for (let i = 1; i < 4; i++) {
    ctx.strokeStyle = `rgba(255,255,255,${(0.08 + (1 - dampN) * 0.08).toFixed(2)})`;
    ctx.beginPath();
    ctx.moveTo(0, (h / 4) * i);
    ctx.lineTo(w, (h / 4) * i);
    ctx.stroke();
  }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function formatFreq(hz: number): string {
  if (hz >= 10000) return `${(hz / 1000).toFixed(0)}k`;
  if (hz >= 1000)  return `${(hz / 1000).toFixed(1)}k`;
  return `${Math.round(hz)}Hz`;
}

function fmtDb(db: number): string {
  if (db <= -59) return "-∞";
  return `${db >= 0 ? "+" : ""}${db.toFixed(1)}`;
}

// ── Root ─────────────────────────────────────────────────────────────────────

export function UltraVerbEditor({ params, enabled, onParamsChange, onToggleEnabled, onReset }: Props) {
  const model    = normalizeUltraVerbParams(params);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const modelRef  = useRef(model);
  modelRef.current = model;

  // Redraw when relevant params change
  useEffect(() => {
    if (canvasRef.current) paintTail(canvasRef.current, model);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [model.decay, model.size, model.mix, model.width, model.diffusion, model.damping, model.modulation]);

  // Redraw on resize
  useEffect(() => {
    const el = canvasRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      if (canvasRef.current) paintTail(canvasRef.current, modelRef.current);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const update = (patch: Partial<UltraVerbParams>) =>
    onParamsChange(serializeUltraVerbParams({ ...model, ...patch }));

  return (
    <div
      className="flex flex-col overflow-hidden"
      style={{
        width: 960,
        height: 420,
        borderRadius: 16,
        border: "1px solid rgba(255,255,255,0.14)",
        background: "linear-gradient(180deg, rgba(255,255,255,0.045) 0%, transparent 34%), linear-gradient(180deg, #111722 0%, #0b0e14 100%)",
        boxShadow: "0 28px 80px rgba(0,0,0,0.5), inset 0 1px 0 rgba(255,255,255,0.06)",
        fontFamily: "Inter, ui-sans-serif, system-ui, sans-serif",
        color: "#edf2ff",
        fontSize: 11,
      }}
    >
      {/* ── Header ── */}
      <div
        className="grid shrink-0 items-center gap-3.5 px-3.5"
        style={{
          gridTemplateColumns: "230px 1fr 250px",
          height: 54,
          background: "rgba(0,0,0,0.18)",
          borderBottom: "1px solid rgba(255,255,255,0.085)",
        }}
      >
        {/* Brand */}
        <div className="flex min-w-0 items-center gap-2.5">
          <PowerButton enabled={enabled && Boolean(model.power)} onToggle={onToggleEnabled} />
          <div className="min-w-0">
            <div style={{ color: "#5e6879", fontSize: 9, fontWeight: 750, letterSpacing: "0.22em", textTransform: "uppercase" }}>
              Futureboard FX
            </div>
            <div style={{ color: "#edf2ff", fontSize: 16, fontWeight: 820, lineHeight: 1, marginTop: 2 }}>
              UltraVerb
            </div>
          </div>
        </div>

        {/* Mode segment */}
        <div className="flex min-w-0 justify-center">
          <div
            className="flex gap-0.5 p-[3px]"
            style={{ border: "1px solid rgba(255,255,255,0.085)", borderRadius: 10, background: "rgba(0,0,0,0.22)" }}
          >
            {MODES.map((m) => {
              const active = model.mode === m.id;
              return (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => update({ mode: m.id })}
                  style={{
                    minWidth: 72,
                    height: 26,
                    borderRadius: 7,
                    fontSize: 11,
                    fontWeight: 720,
                    cursor: "pointer",
                    border: "none",
                    transition: "all 0.1s",
                    background: active
                      ? "linear-gradient(180deg, rgba(124,199,255,0.22), rgba(124,199,255,0.1))"
                      : "transparent",
                    color:     active ? "#f7fbff" : "#8b95a8",
                    boxShadow: active ? "inset 0 0 0 1px rgba(124,199,255,0.28)" : "none",
                  }}
                >
                  {m.label}
                </button>
              );
            })}
          </div>
        </div>

        {/* Preset row */}
        <div className="flex min-w-0 items-center justify-end gap-2">
          <div
            className="flex shrink-0 items-center justify-between px-2.5"
            style={{
              width: 158,
              height: 28,
              border: "1px solid rgba(255,255,255,0.085)",
              borderRadius: 9,
              background: "rgba(255,255,255,0.035)",
              color: "#8b95a8",
              fontSize: 11,
              fontWeight: 680,
            }}
          >
            <span>Wide Hall A</span>
            <span style={{ fontSize: 8 }}>▾</span>
          </div>
          <MiniBtn>A/B</MiniBtn>
          <MiniBtn onClick={onReset}>R</MiniBtn>
        </div>
      </div>

      {/* ── Main ── */}
      <div className="grid min-h-0 flex-1 gap-2.5 p-2.5" style={{ gridTemplateColumns: "1.35fr 1fr 204px" }}>

        {/* Visual panel */}
        <Panel className="grid overflow-hidden" style={{ gridTemplateRows: "34px 1fr 48px" }}>
          <div
            className="flex items-center justify-between px-3"
            style={{ borderBottom: "1px solid rgba(255,255,255,0.055)" }}
          >
            <PanelTitle>Tail Shape</PanelTitle>
            <span style={{ color: "#edf2ff", fontSize: 17, fontWeight: 820, fontVariantNumeric: "tabular-nums" }}>
              {model.decay.toFixed(2)}s
            </span>
          </div>

          <div
            className="relative min-h-0"
            style={{ background: "radial-gradient(circle at 50% 55%, rgba(124,199,255,0.14), transparent 50%), #080b11" }}
          >
            <canvas ref={canvasRef} className="absolute inset-0 h-full w-full" />
            <div
              className="pointer-events-none absolute inset-x-3 top-3 flex justify-between"
              style={{ color: "rgba(237,242,255,0.42)", fontSize: 10, fontWeight: 680 }}
            >
              <span>Early reflections</span>
              <span>Late field</span>
            </div>
          </div>

          <div
            className="grid gap-2 p-2"
            style={{
              gridTemplateColumns: "repeat(4,1fr)",
              borderTop: "1px solid rgba(255,255,255,0.055)",
              background: "rgba(0,0,0,0.16)",
            }}
          >
            {([
              ["Size",    `${model.size.toFixed(0)}%`],
              ["Density", `${model.diffusion.toFixed(0)}%`],
              ["Damp",    `${model.damping.toFixed(0)}%`],
              ["Width",   `${model.width.toFixed(0)}%`],
            ] as [string, string][]).map(([label, val]) => (
              <div
                key={label}
                className="flex flex-col gap-[3px] rounded-lg px-2 py-1.5"
                style={{ background: "rgba(255,255,255,0.035)" }}
              >
                <span style={{ color: "#5e6879", fontSize: 9, fontWeight: 760, letterSpacing: "0.12em", textTransform: "uppercase" }}>
                  {label}
                </span>
                <span style={{ color: "#edf2ff", fontSize: 12, fontWeight: 760, fontVariantNumeric: "tabular-nums" }}>
                  {val}
                </span>
              </div>
            ))}
          </div>
        </Panel>

        {/* Controls panel */}
        <div className="grid min-h-0 gap-2.5" style={{ gridTemplateRows: "90px 1fr 76px" }}>

          {/* Macro knobs */}
          <div className="grid gap-2" style={{ gridTemplateColumns: "repeat(4,1fr)" }}>
            <KnobCard
              label="Size"  value={`${model.size.toFixed(0)}%`}
              pct={model.size}
              onDrag={(d) => update({ size: clamp(model.size + d * 0.6, 0, 100) })}
            />
            <KnobCard
              label="Decay" value={`${model.decay.toFixed(2)}s`}
              pct={((model.decay - 0.1) / 19.9) * 100}
              onDrag={(d) => update({ decay: clamp(model.decay + d * 0.08, 0.1, 20) })}
            />
            <KnobCard
              label="Mix"   value={`${model.mix.toFixed(0)}%`}
              pct={model.mix}
              onDrag={(d) => update({ mix: clamp(model.mix + d * 0.6, 0, 100) })}
            />
            <KnobCard
              label="Width" value={`${model.width.toFixed(0)}%`}
              pct={(model.width / 150) * 100}
              onDrag={(d) => update({ width: clamp(model.width + d * 0.8, 0, 150) })}
            />
          </div>

          {/* Section grid */}
          <div className="grid min-h-0 gap-2.5" style={{ gridTemplateColumns: "repeat(2,minmax(0,1fr))" }}>

            <Panel className="flex min-h-0 flex-col gap-2 p-2.5">
              <SectionHead label="Space">
                <MiniToggle options={["Eco", "HQ"]} active={1} />
              </SectionHead>
              <SliderRow
                label="Pre"
                value={`${model.preDelayMs.toFixed(0)}ms`}
                pct={model.preDelayMs / 250}
                onChange={(p) => update({ preDelayMs: clamp(p * 250, 0, 250) })}
              />
              <SliderRow
                label="Diffuse"
                value={`${model.diffusion.toFixed(0)}%`}
                pct={model.diffusion / 100}
                onChange={(p) => update({ diffusion: clamp(p * 100, 0, 100) })}
              />
              <SliderRow
                label="Early"
                value={fmtDb(model.earlyLevel)}
                pct={(model.earlyLevel + 60) / 66}
                onChange={(p) => update({ earlyLevel: clamp(p * 66 - 60, -60, 6) })}
              />
              <SliderRow
                label="Late"
                value={fmtDb(model.lateLevel)}
                pct={(model.lateLevel + 60) / 66}
                onChange={(p) => update({ lateLevel: clamp(p * 66 - 60, -60, 6) })}
              />
            </Panel>

            <Panel className="flex min-h-0 flex-col gap-2 p-2.5">
              <SectionHead label="Tone">
                <MiniToggle options={["Post", "Pre"]} active={0} />
              </SectionHead>
              <SliderRow
                label="Lo Cut"
                value={formatFreq(model.lowCutHz)}
                pct={Math.log(model.lowCutHz / 20) / Math.log(50)}
                onChange={(p) => update({ lowCutHz: clamp(20 * Math.pow(50, p), 20, 1000) })}
              />
              <SliderRow
                label="Hi Cut"
                value={formatFreq(model.highCutHz)}
                pct={(model.highCutHz - 1000) / 19000}
                onChange={(p) => update({ highCutHz: clamp(p * 19000 + 1000, 1000, 20000) })}
              />
              <SliderRow
                label="Damp"
                value={`${model.damping.toFixed(0)}%`}
                pct={model.damping / 100}
                onChange={(p) => update({ damping: clamp(p * 100, 0, 100) })}
              />
              <SliderRow
                label="Motion"
                value={`${model.modulation.toFixed(0)}%`}
                pct={model.modulation / 100}
                onChange={(p) => update({ modulation: clamp(p * 100, 0, 100) })}
              />
            </Panel>
          </div>

          {/* Bottom row */}
          <div className="grid gap-2.5" style={{ gridTemplateColumns: "1fr 1fr 112px" }}>
            <Panel>
              <SwitchCard
                label="Freeze"
                active={model.freeze}
                onClick={() => update({ freeze: !model.freeze })}
              />
            </Panel>
            <Panel>
              <SwitchCard
                label="Stereo Field"
                active={model.width > 100}
                onClick={() => update({ width: model.width > 100 ? 100 : 110 })}
              />
            </Panel>
            <Panel className="grid grid-cols-2 gap-1.5 p-2">
              <RouteBtn active>INS</RouteBtn>
              <RouteBtn active={false}>SEND</RouteBtn>
            </Panel>
          </div>
        </div>

        {/* Side panel */}
        <div className="grid min-h-0 gap-2.5" style={{ gridTemplateRows: "1fr 104px" }}>

          <Panel className="flex min-h-0 flex-col gap-2.5 p-2.5">
            <SectionHead label="Level">
              <span style={{ color: "#8b95a8", fontSize: 10, fontWeight: 760, fontVariantNumeric: "tabular-nums" }}>
                {fmtDb(model.outputDb)} dB
              </span>
            </SectionHead>
            <MeterRow label="IN L" pct={0.72} display="-8.2" />
            <MeterRow label="IN R" pct={0.68} display="-8.9" />
            <MeterRow label="WET"  pct={clamp(model.mix / 100 * 0.6, 0, 1)} display={fmtDb(model.lateLevel)} />
            <div
              className="flex flex-1 flex-col items-center justify-center rounded-[10px]"
              style={{ border: "1px solid rgba(255,255,255,0.085)", background: "rgba(0,0,0,0.16)" }}
            >
              <div style={{ color: "#edf2ff", fontSize: 28, lineHeight: 1, fontWeight: 830, fontVariantNumeric: "tabular-nums" }}>
                {model.outputDb >= 0 ? "+" : ""}{model.outputDb.toFixed(1)}
              </div>
              <div style={{ color: "#5e6879", fontSize: 9, fontWeight: 780, letterSpacing: "0.14em", textTransform: "uppercase", marginTop: 6 }}>
                Output dB
              </div>
            </div>
          </Panel>

          <Panel className="flex flex-col gap-2.5 p-2.5">
            <SectionHead label="Output" />
            <SliderRow
              label="Trim"
              value={`${model.outputDb >= 0 ? "+" : ""}${model.outputDb.toFixed(1)}`}
              pct={(model.outputDb + 24) / 36}
              onChange={(p) => update({ outputDb: clamp(p * 36 - 24, -24, 12) })}
            />
            <SliderRow
              label="Duck"
              value="0%"
              pct={0}
              onChange={() => {}}
            />
          </Panel>
        </div>
      </div>
    </div>
  );
}

// ── Sub-components ───────────────────────────────────────────────────────────

function Panel({ children, className, style }: {
  children?: ReactNode;
  className?: string;
  style?: React.CSSProperties;
}) {
  return (
    <div
      className={`min-h-0 min-w-0 ${className ?? ""}`}
      style={{
        border: "1px solid rgba(255,255,255,0.085)",
        borderRadius: 10,
        background: "linear-gradient(180deg, rgba(255,255,255,0.035), rgba(255,255,255,0.014)), #141a24",
        boxShadow: "inset 0 1px 0 rgba(255,255,255,0.04)",
        ...style,
      }}
    >
      {children}
    </div>
  );
}

function PanelTitle({ children }: { children: ReactNode }) {
  return (
    <span style={{ color: "#8b95a8", fontSize: 10, fontWeight: 780, letterSpacing: "0.14em", textTransform: "uppercase" }}>
      {children}
    </span>
  );
}

function SectionHead({ label, children }: { label: string; children?: ReactNode }) {
  return (
    <div className="flex items-center justify-between">
      <span style={{ color: "#8b95a8", fontSize: 10, fontWeight: 800, letterSpacing: "0.12em", textTransform: "uppercase" }}>
        {label}
      </span>
      {children}
    </div>
  );
}

function PowerButton({ enabled, onToggle }: { enabled: boolean; onToggle: () => void }) {
  return (
    <button
      type="button"
      onClick={onToggle}
      title={enabled ? "Bypass" : "Enable"}
      className="grid shrink-0 place-items-center"
      style={{
        width: 28, height: 28,
        borderRadius: 8,
        fontSize: 13, fontWeight: 800,
        cursor: "pointer",
        border: enabled ? "1px solid rgba(124,199,255,0.35)" : "1px solid rgba(255,255,255,0.12)",
        background: enabled ? "rgba(124,199,255,0.14)" : "rgba(255,255,255,0.035)",
        color: enabled ? "#7cc7ff" : "#5e6879",
        boxShadow: enabled
          ? "inset 0 1px 0 rgba(255,255,255,0.08), 0 0 12px rgba(124,199,255,0.2)"
          : "inset 0 1px 0 rgba(255,255,255,0.04)",
        transition: "all 0.15s",
      }}
    >
      I
    </button>
  );
}

function MiniBtn({ children, onClick }: { children: ReactNode; onClick?: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="grid shrink-0 place-items-center"
      style={{
        width: 28, height: 28,
        borderRadius: 8,
        border: "1px solid rgba(255,255,255,0.085)",
        background: "rgba(255,255,255,0.035)",
        color: "#8b95a8",
        cursor: "pointer",
        fontSize: 11, fontWeight: 800,
      }}
    >
      {children}
    </button>
  );
}

function KnobCard({ label, value, pct, onDrag }: {
  label: string;
  value: string;
  pct: number;
  onDrag: (delta: number) => void;
}) {
  const dragRef = useRef<{ y: number } | null>(null);
  const p = clamp(pct, 0, 100);

  return (
    <div
      className="grid place-items-center gap-1 rounded-[10px]"
      style={{
        border: "1px solid rgba(255,255,255,0.085)",
        background: "rgba(255,255,255,0.03)",
        padding: "7px 4px 6px",
      }}
    >
      <div
        className="relative shrink-0 cursor-ns-resize rounded-full"
        style={{
          width: 38, height: 38,
          background: `conic-gradient(from -135deg, #7cc7ff ${p * 2.7}deg, rgba(255,255,255,0.1) 0 270deg, transparent 0), linear-gradient(145deg, #1b2330, #090c12)`,
          boxShadow: "inset 0 1px 0 rgba(255,255,255,0.12), 0 8px 16px rgba(0,0,0,0.3)",
        }}
        onPointerDown={(e) => {
          dragRef.current = { y: e.clientY };
          e.currentTarget.setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => {
          if (!dragRef.current) return;
          onDrag(dragRef.current.y - e.clientY);
          dragRef.current = { y: e.clientY };
        }}
        onPointerUp={(e) => {
          dragRef.current = null;
          e.currentTarget.releasePointerCapture(e.pointerId);
        }}
      >
        <div
          className="pointer-events-none absolute left-1/2 rounded-full"
          style={{
            width: 2, height: 13,
            top: 7,
            background: "#f8fbff",
            transformOrigin: "50% 12px",
            transform: `translateX(-50%) rotate(${-135 + p * 2.7}deg)`,
          }}
        />
      </div>
      <span style={{ color: "#5e6879", fontSize: 9, fontWeight: 780, letterSpacing: "0.12em", textTransform: "uppercase" }}>
        {label}
      </span>
      <span style={{ color: "#edf2ff", fontSize: 11, fontWeight: 760, fontVariantNumeric: "tabular-nums" }}>
        {value}
      </span>
    </div>
  );
}

function SliderRow({ label, value, pct, onChange }: {
  label: string;
  value: string;
  pct: number;
  onChange: (p: number) => void;
}) {
  const trackRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);
  const p = clamp(pct, 0, 1);

  const seek = (clientX: number) => {
    if (!trackRef.current) return;
    const rect = trackRef.current.getBoundingClientRect();
    onChange(clamp((clientX - rect.left) / rect.width, 0, 1));
  };

  return (
    <div className="grid items-center gap-2" style={{ gridTemplateColumns: "66px 1fr 54px" }}>
      <span style={{ color: "#8b95a8", fontSize: 11, fontWeight: 680 }}>{label}</span>
      <div
        ref={trackRef}
        className="relative cursor-ew-resize rounded-full"
        style={{
          height: 4,
          background: `linear-gradient(90deg, #7cc7ff ${p * 100}%, rgba(255,255,255,0.11) ${p * 100}%)`,
        }}
        onPointerDown={(e) => {
          dragging.current = true;
          e.currentTarget.setPointerCapture(e.pointerId);
          seek(e.clientX);
        }}
        onPointerMove={(e) => { if (dragging.current) seek(e.clientX); }}
        onPointerUp={() => { dragging.current = false; }}
        onPointerCancel={() => { dragging.current = false; }}
      >
        <div
          className="pointer-events-none absolute top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full"
          style={{
            left: `${p * 100}%`,
            width: 13, height: 13,
            border: "2px solid #121821",
            background: "#f8fbff",
            boxShadow: "0 0 0 4px rgba(124,199,255,0.12)",
          }}
        />
      </div>
      <span
        className="text-right"
        style={{ color: "#edf2ff", fontSize: 11, fontWeight: 760, fontVariantNumeric: "tabular-nums" }}
      >
        {value}
      </span>
    </div>
  );
}

function MiniToggle({ options, active }: { options: string[]; active: number }) {
  return (
    <div
      className="flex overflow-hidden rounded-lg"
      style={{ border: "1px solid rgba(255,255,255,0.085)", background: "rgba(0,0,0,0.2)" }}
    >
      {options.map((opt, i) => (
        <button
          key={opt}
          type="button"
          style={{
            height: 22, minWidth: 39,
            background: i === active ? "rgba(124,199,255,0.18)" : "transparent",
            color: i === active ? "#edf2ff" : "#5e6879",
            fontSize: 10, fontWeight: 760,
            cursor: "pointer",
            border: "none",
          }}
        >
          {opt}
        </button>
      ))}
    </div>
  );
}

function SwitchCard({ label, active, onClick }: { label: string; active: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      className="flex h-full w-full items-center justify-between gap-2.5 px-2.5"
      style={{ cursor: "pointer", background: "none", border: "none" }}
      onClick={onClick}
    >
      <span style={{ color: "#8b95a8", fontSize: 11, fontWeight: 760 }}>{label}</span>
      <div
        className="relative shrink-0 rounded-full"
        style={{
          width: 42, height: 22,
          border: active ? "1px solid rgba(124,199,255,0.28)" : "1px solid rgba(255,255,255,0.085)",
          background: active ? "rgba(124,199,255,0.12)" : "rgba(0,0,0,0.2)",
          transition: "all 0.15s",
        }}
      >
        <div
          className="absolute top-[3px] rounded-full"
          style={{
            width: 14, height: 14,
            right: active ? 3 : "auto",
            left:  active ? "auto" : 3,
            background: active ? "#7cc7ff" : "#3a4555",
            boxShadow: active ? "0 0 14px rgba(124,199,255,0.45)" : "none",
            transition: "all 0.15s",
          }}
        />
      </div>
    </button>
  );
}

function RouteBtn({ children, active }: { children: ReactNode; active: boolean }) {
  return (
    <button
      type="button"
      className="rounded-[9px]"
      style={{
        border: active ? "1px solid rgba(124,199,255,0.35)" : "1px solid rgba(255,255,255,0.085)",
        background: active ? "rgba(124,199,255,0.12)" : "rgba(255,255,255,0.035)",
        color: active ? "#edf2ff" : "#8b95a8",
        fontSize: 10, fontWeight: 780,
        cursor: "pointer",
      }}
    >
      {children}
    </button>
  );
}

function MeterRow({ label, pct, display }: { label: string; pct: number; display: string }) {
  return (
    <div className="grid items-center gap-2" style={{ gridTemplateColumns: "28px 1fr 42px" }}>
      <span style={{ color: "#8b95a8", fontSize: 10, fontWeight: 800, fontVariantNumeric: "tabular-nums" }}>
        {label}
      </span>
      <div className="overflow-hidden rounded-full" style={{ height: 7, background: "rgba(255,255,255,0.075)" }}>
        <div
          className="h-full rounded-full"
          style={{
            width: `${clamp(pct * 100, 0, 100)}%`,
            background: "linear-gradient(90deg, #85f3b4, #ffd37a 68%, #ff7d8a)",
          }}
        />
      </div>
      <span
        className="text-right"
        style={{ color: "#8b95a8", fontSize: 10, fontWeight: 800, fontVariantNumeric: "tabular-nums" }}
      >
        {display}
      </span>
    </div>
  );
}
