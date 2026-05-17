import { useEffect, useRef } from "react";
import { activeAudioEngine } from "../../engine/activeAudioEngine";
import { useTransportStore } from "../../store/transportStore";
import { useUIStore } from "../../store/uiStore";
import { C } from "../../theme";
import { TIMELINE_CONTENT_LEFT, timeToContentX } from "../../utils/musicalTime";
import { TIMELINE_Z } from "../../utils/timelineZ";

/**
 * Renders the playhead inside a clip container that starts at the timeline
 * content origin (TIMELINE_CONTENT_LEFT = HEADER_WIDTH).  Both the line and
 * the marker share the same parent, the same z-index, and the same x
 * derivation (`timeToContentX`) — so they can never separate, and they can
 * never visually leak across the sticky track-header lane.
 *
 * Coordinates inside the wrapper are CONTENT pixels:
 *   line/marker x  =  timeToContentX(t, pps, scrollX)
 *
 * The vertical line is 2 px wide; both line and marker centre on the same
 * pixel column the canvas grid draws (Math.round(x) + 0.5).
 */
const LINE_W = 2;
const HEAD_W = 12;

export function Playhead() {
  const lineRef  = useRef<HTMLDivElement>(null);
  const headRef  = useRef<HTMLDivElement>(null);
  const rafRef   = useRef<number>(0);
  const lastStore = useRef(0);
  const setPlayheadTime = useTransportStore((s) => s.setPlayheadTime);

  useEffect(() => {
    const tick = () => {
      const { pixelsPerSecond: pps, scrollX, loopEnabled, loopStart, loopEnd } =
        useUIStore.getState();
      const t = activeAudioEngine.projectTime;

      if (activeAudioEngine.isPlaying && loopEnabled && t >= loopEnd) {
        activeAudioEngine.seekSeconds(loopStart);
        rafRef.current = requestAnimationFrame(tick);
        return;
      }

      // Content-area x — wrapper already begins at TIMELINE_CONTENT_LEFT,
      // so the line/marker never paint over the sticky track-header lane.
      const x = timeToContentX(t, pps, scrollX);

      // Line: 2-px wide, centred on the canvas grid pixel column.
      if (lineRef.current) lineRef.current.style.transform = `translateX(${x - LINE_W / 2}px)`;
      // Triangle: 12-px wide, centred on the same column as the line.
      if (headRef.current) headRef.current.style.transform = `translateX(${x - HEAD_W / 2}px)`;

      const now = performance.now();
      if (now - lastStore.current > 100) {
        setPlayheadTime(Math.round(t * 100) / 100);
        lastStore.current = now;
      }
      rafRef.current = requestAnimationFrame(tick);
    };

    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [setPlayheadTime]);

  return (
    // Clip container — starts at TIMELINE_CONTENT_LEFT, overflow-hidden so
    // the line and marker can never visually paint across the sticky
    // track-header lane on the left.  Single z-index for both children.
    <div
      className="pointer-events-none absolute top-0 bottom-0 right-0 overflow-hidden"
      style={{ left: TIMELINE_CONTENT_LEFT, zIndex: TIMELINE_Z.playhead }}
      aria-hidden
    >
      {/* Triangle marker — sits inside the ruler area */}
      <div
        ref={headRef}
        className="absolute left-0 top-0 pointer-events-none will-change-transform"
      >
        <svg width={HEAD_W} height={12} viewBox="0 0 12 12" className="block drop-shadow">
          <polygon points="0,0 12,0 6,12" fill={C.accent} />
        </svg>
      </div>

      {/* Vertical line — spans ruler top through all track rows */}
      <div
        ref={lineRef}
        className="absolute left-0 top-0 bottom-0 pointer-events-none will-change-transform"
        style={{ width: LINE_W, background: C.playhead + "cc" }}
      />
    </div>
  );
}
