import { useEffect, useRef } from "react";
import { transport } from "../../engine/Transport";
import { useTransportStore } from "../../store/transportStore";
import { useUIStore } from "../../store/uiStore";
import { HEADER_WIDTH } from "../../theme";

export function Playhead({ height }: { height: number }) {
  const lineRef = useRef<HTMLDivElement>(null);
  const headRef = useRef<HTMLDivElement>(null);
  const rafRef  = useRef<number>(0);
  const lastStore = useRef(0);
  const { pixelsPerSecond } = useUIStore();
  const setPlayheadTime = useTransportStore((s) => s.setPlayheadTime);

  useEffect(() => {
    const tick = () => {
      const t = transport.projectTime;
      const { loopEnabled, loopStart, loopEnd } = useUIStore.getState();

      if (transport.isPlaying && loopEnabled && t >= loopEnd) {
        transport.seek(loopStart);
        rafRef.current = requestAnimationFrame(tick);
        return;
      }

      const x = HEADER_WIDTH + t * pixelsPerSecond;
      if (lineRef.current) lineRef.current.style.transform = `translateX(${x}px)`;
      if (headRef.current) headRef.current.style.transform = `translateX(${x - 4}px)`;
      const now = performance.now();
      if (now - lastStore.current > 100) { setPlayheadTime(Math.round(t * 100) / 100); lastStore.current = now; }
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [pixelsPerSecond, setPlayheadTime]);

  return (
    <>
      {/* Playhead triangle (sits on ruler) */}
      <div ref={headRef} className="absolute top-0 left-0 pointer-events-none z-30 will-change-transform">
        <svg width={12} height={12} viewBox="0 0 12 12" className="block drop-shadow">
          <polygon points="0,0 12,0 6,12" fill="#48a6a7" />
        </svg>
      </div>
      {/* Vertical line through tracks */}
      <div
        ref={lineRef}
        className="absolute left-0 top-0 pointer-events-none z-30 will-change-transform"
        style={{ width: 2, height, background: "rgba(72,166,167,0.86)" }}
      />
    </>
  );
}
