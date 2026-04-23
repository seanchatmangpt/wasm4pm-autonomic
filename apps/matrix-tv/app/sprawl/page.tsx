'use client';

import { Canvas } from '@react-three/fiber';
import Link from 'next/link';
import { useEffect, useMemo, useState } from 'react';
import { Globe } from '@/components/GlobeRenderer';
import { QuestHud } from '@/components/QuestHud';
import { ReceiptRibbon } from '@/components/ReceiptRibbon';
import { VerdictBadge } from '@/components/VerdictBadge';
import { eventToMotionResponse, loadReplay, type SprawlEvent } from '@/lib/sprawl';

/**
 * `/sprawl` — Blockchain MUD replay.
 *
 * Loads `/sprawl-replay.ndjson` (produced by `cargo run -p unibit-sprawl
 * --bin sprawl -- walk`) and steps through the canonical Neuromancer
 * quest arc, painting each admitted motion onto the 64³ globe and
 * stacking each DualReceipt onto the ribbon.
 */
export default function SprawlPage() {
  const [events, setEvents] = useState<SprawlEvent[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [index, setIndex] = useState(0);
  const [playing, setPlaying] = useState(true);

  useEffect(() => {
    loadReplay().then(setEvents).catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    if (!playing || !events || events.length === 0) return;
    const iv = setInterval(() => {
      setIndex((i) => (i + 1 < events.length ? i + 1 : i));
    }, 900);
    return () => clearInterval(iv);
  }, [playing, events]);

  const response = useMemo(() => {
    if (!events || events.length === 0) return null;
    const ev = events[Math.min(index, events.length - 1)];
    return eventToMotionResponse(ev);
  }, [events, index]);

  const history = useMemo(() => {
    if (!events) return [];
    return events.slice(0, index + 1).map(eventToMotionResponse);
  }, [events, index]);

  if (error) {
    return (
      <main style={{ padding: 40, color: '#e63333', fontFamily: 'ui-monospace, monospace' }}>
        <h1>Sprawl replay failed to load</h1>
        <p>{error}</p>
        <p style={{ opacity: 0.7, fontSize: 12 }}>
          Generate the replay with:{' '}
          <code>cargo run -p unibit-sprawl --bin sprawl -- walk &gt; apps/matrix-tv/public/sprawl-replay.ndjson</code>
        </p>
      </main>
    );
  }

  if (!events || !response) {
    return (
      <main style={{ padding: 40, color: '#999' }}>
        <p>loading sprawl replay…</p>
      </main>
    );
  }

  const atEnd = index >= events.length - 1;

  return (
    <main
      data-testid="sprawl-page"
      data-event-count={events.length}
      data-current-index={index}
      style={{ padding: 0, position: 'relative', minHeight: '100vh' }}
    >
      <header
        style={{
          position: 'absolute',
          top: 24,
          left: 24,
          zIndex: 10,
          padding: 16,
          background: 'rgba(12, 15, 24, 0.85)',
          border: '1px solid #333',
          borderRadius: 8,
          maxWidth: 320,
        }}
      >
        <Link href="/" style={{ color: '#9ab', textDecoration: 'none', fontSize: 12 }}>
          ← back
        </Link>
        <div style={{ fontSize: 10, opacity: 0.5, marginTop: 4, letterSpacing: 1 }}>
          SPRAWL · blockchain MUD
        </div>
        <h1 style={{ fontSize: 22, margin: '4px 0 8px 0' }}>
          Case → Loa
        </h1>
        <div style={{ fontSize: 12, opacity: 0.7, marginBottom: 12 }}>
          Nine Neuromancer character-gates as MUD rooms. Every admitted motion
          advances the DualReceipt chain — the verifiable history of the world.
        </div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <button
            data-testid="play-pause"
            onClick={() => setPlaying((p) => !p)}
            style={buttonStyle}
            type="button"
          >
            {playing ? '❚❚' : '▶'}
          </button>
          <button
            data-testid="reset"
            onClick={() => {
              setIndex(0);
              setPlaying(true);
            }}
            style={buttonStyle}
            type="button"
          >
            ⏮
          </button>
          <input
            data-testid="scrub"
            type="range"
            min={0}
            max={events.length - 1}
            value={index}
            onChange={(e) => {
              setIndex(Number(e.target.value));
              setPlaying(false);
            }}
            style={{ flex: 1 }}
          />
        </div>
        {atEnd && (
          <div style={{ marginTop: 8, fontSize: 11, color: '#33cc4d' }}>
            ✓ chain sealed — {events.length} turns, all Lawful
          </div>
        )}
      </header>

      <div data-testid="sprawl-canvas-wrapper" style={{ width: '100%', height: '100%' }}>
        <Canvas
          camera={{ position: [0, 0, 6], fov: 50 }}
          gl={{ preserveDrawingBuffer: true }}
        >
          <ambientLight intensity={0.4} />
          <pointLight position={[5, 5, 5]} intensity={0.6} />
          <Globe response={response} />
        </Canvas>
      </div>

      <VerdictBadge response={response} />
      <QuestHud events={events} currentIndex={index} />
      <ReceiptRibbon history={history} />
    </main>
  );
}

const buttonStyle: React.CSSProperties = {
  padding: '6px 12px',
  background: 'rgba(20, 20, 30, 0.9)',
  color: '#e8e8f0',
  border: '1px solid #444',
  borderRadius: 4,
  cursor: 'pointer',
  fontFamily: 'ui-monospace, monospace',
  letterSpacing: 1,
  fontSize: 12,
  minWidth: 36,
};
