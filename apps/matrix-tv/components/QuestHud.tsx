'use client';

import type { SprawlEvent } from '@/lib/sprawl';
import { ROOM_COLOR } from '@/lib/sprawl';

/**
 * Heads-up display for the Sprawl MUD replay.
 *
 * Shows the current tick, room, verb, verdict, scope, delta popcount,
 * and a compact receipt-chain trail.
 */
export function QuestHud({
  events,
  currentIndex,
}: {
  events: SprawlEvent[];
  currentIndex: number;
}) {
  const current = events[currentIndex];
  if (!current) return null;

  const trail = events.slice(0, currentIndex + 1);
  const lawfulCount = trail.filter((e) => e.verdict === 'Lawful').length;

  return (
    <div
      data-testid="quest-hud"
      style={{
        position: 'absolute',
        top: 24,
        right: 24,
        zIndex: 20,
        width: 320,
        padding: 16,
        background: 'rgba(12, 15, 24, 0.85)',
        border: '1px solid #333',
        borderRadius: 8,
        color: '#e8e8f0',
        fontFamily: 'ui-monospace, monospace',
        fontSize: 12,
        letterSpacing: 0.5,
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between' }}>
        <span style={{ opacity: 0.6 }}>TICK</span>
        <span>{current.tick}</span>
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 4 }}>
        <span style={{ opacity: 0.6 }}>ROOM</span>
        <span style={{ color: ROOM_COLOR[current.room] ?? '#fff' }}>
          {current.room.toUpperCase()}
        </span>
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 4 }}>
        <span style={{ opacity: 0.6 }}>VERB</span>
        <span>{current.verb}</span>
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 4 }}>
        <span style={{ opacity: 0.6 }}>SCOPE</span>
        <span>#{current.scope}</span>
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 4 }}>
        <span style={{ opacity: 0.6 }}>VERDICT</span>
        <span
          style={{
            color: current.verdict === 'Lawful' ? '#33cc4d' : '#e63333',
          }}
        >
          {current.verdict}
        </span>
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 4 }}>
        <span style={{ opacity: 0.6 }}>Δ POPCOUNT</span>
        <span>{current.delta_popcount}</span>
      </div>

      <div
        style={{
          marginTop: 12,
          paddingTop: 12,
          borderTop: '1px solid #333',
          fontSize: 10,
          opacity: 0.7,
        }}
      >
        chain depth: {trail.length} · lawful: {lawfulCount}/{trail.length}
      </div>

      <div
        data-testid="room-trail"
        style={{
          marginTop: 8,
          display: 'flex',
          gap: 2,
          flexWrap: 'wrap',
        }}
      >
        {events.map((ev, i) => (
          <span
            key={i}
            title={`${ev.tick}: ${ev.room} ${ev.verb} (${ev.verdict})`}
            style={{
              width: 10,
              height: 10,
              borderRadius: 2,
              background:
                i > currentIndex
                  ? '#222'
                  : ev.verdict === 'Lawful'
                    ? ROOM_COLOR[ev.room] ?? '#4db2ff'
                    : '#e63333',
              border: i === currentIndex ? '1px solid #fff' : '1px solid transparent',
            }}
          />
        ))}
      </div>
    </div>
  );
}
