import { memo, useEffect, useRef } from "react";

// ── Segment config ─────────────────────────────────────────────────────────────

const SEGMENTS = 20;
const SEG_GAP  = 1.5; // px between segments

// Pre-allocate color lookup — no allocation on render/draw
const ON_COLORS: string[] = Array.from({ length: SEGMENTS }, (_, i) => {
  if (i >= SEGMENTS - 2)  return "#e9756e";
  if (i >= SEGMENTS - 5)  return "#e8be58";
  if (i >= SEGMENTS - 10) return "#56c7c9";
  return "#3a9fa1";
});
const OFF_COLOR = "rgba(255,255,255,0.045)";

// ── Draw helper ────────────────────────────────────────────────────────────────

function drawColumn(
  ctx: CanvasRenderingContext2D,
  level: number,
  x: number,
  colW: number,
  h: number,
): void {
  const clamped = level < 0 ? 0 : level > 1 ? 1 : level;
  const active  = Math.round(clamped * SEGMENTS);
  const segH    = (h - SEG_GAP * (SEGMENTS - 1)) / SEGMENTS;

  for (let i = 0; i < SEGMENTS; i++) {
    ctx.fillStyle = i < active ? ON_COLORS[i] : OFF_COLOR;
    // i = 0 → bottom segment
    const y = h - (i + 1) * segH - i * SEG_GAP;
    ctx.beginPath();
    ctx.roundRect(x, y, colW, segH, 1);
    ctx.fill();
  }
}

// ── Component ──────────────────────────────────────────────────────────────────

type Props = {
  mode?: "mono" | "stereo";
  levelL: number;
  levelR: number;
  height?: number;
  /** Width of one meter column. */
  columnWidth?: number;
};

export const VuMeter = memo(function VuMeter({
  mode = "mono",
  levelL,
  levelR,
  height,
  columnWidth = 5,
}: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef    = useRef<HTMLCanvasElement>(null);

  const isStereo = mode === "stereo";
  const colGap   = isStereo ? 2 : 0;
  const canvasW  = isStereo ? columnWidth * 2 + colGap : columnWidth;

  // Draw after every render (meter updates flow as prop changes → re-render → draw).
  useEffect(() => {
    const canvas    = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const dpr = window.devicePixelRatio || 1;
    const w   = canvasW;
    const h   = height ?? container.offsetHeight;
    if (h <= 0) return;

    // Resize backing store only when dimensions actually change.
    const bw = Math.round(w * dpr);
    const bh = Math.round(h * dpr);
    if (canvas.width !== bw || canvas.height !== bh) {
      canvas.width  = bw;
      canvas.height = bh;
    }

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    if (isStereo) {
      drawColumn(ctx, levelL, 0,                    columnWidth, h);
      drawColumn(ctx, levelR, columnWidth + colGap, columnWidth, h);
    } else {
      drawColumn(ctx, Math.max(levelL, levelR), 0, columnWidth, h);
    }
  });

  return (
    <div
      ref={containerRef}
      style={{
        width:    canvasW,
        height:   height,
        flexShrink: 0,
        ...(height === undefined ? { alignSelf: "stretch" } : {}),
      }}
    >
      <canvas
        ref={canvasRef}
        style={{ display: "block", width: canvasW, height: height ?? "100%" }}
      />
    </div>
  );
});
