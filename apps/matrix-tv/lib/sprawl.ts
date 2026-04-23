/**
 * Sprawl MUD replay adapter.
 *
 * Consumes newline-delimited SprawlEvent JSON produced by the Rust
 * `unibit-sprawl` crate and maps each event into the existing
 * MotionResponse shape so the Globe/LaneAxes/ReceiptRibbon components
 * can render it without changes.
 */

import type { FieldLaneName, MotionResponse } from '@/lib/unibit';
import { LANES } from '@/lib/unibit';

export type WireVerdict = 'Lawful' | 'FastOnly' | 'CausalOnly' | 'Unlawful';

export interface SprawlEvent {
  tick: number;
  room: string;
  verb: string;
  verdict: WireVerdict;
  scope: number;
  before_cell: number;
  after_cell: number;
  lane_denies: number[]; // eight entries, one per lane
  delta_popcount: number;
  receipt_fast: number;
  receipt_causal_prefix: number[]; // 16 octets
}

/** Fetch and parse the static replay file shipped in /public/. */
export async function loadReplay(
  url = '/sprawl-replay.ndjson'
): Promise<SprawlEvent[]> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`replay load failed: ${res.status}`);
  const text = await res.text();
  return text
    .split('\n')
    .map((s) => s.trim())
    .filter((s) => s.length > 0)
    .map((line) => JSON.parse(line) as SprawlEvent);
}

/** Adapt a SprawlEvent to a MotionResponse for reuse of existing components. */
export function eventToMotionResponse(ev: SprawlEvent): MotionResponse {
  const perLane = {} as Record<FieldLaneName, bigint>;
  LANES.forEach((lane, i) => {
    perLane[lane] = BigInt(ev.lane_denies[i] ?? 0);
  });
  const denyTotal = ev.verdict === 'Lawful' ? 0n : BigInt(0xffffffff);
  return {
    nextMarking: BigInt(ev.after_cell),
    denyTotal,
    fragment: BigInt(ev.receipt_fast),
    status: ev.verdict === 'Lawful' ? 0 : 1,
    perLane,
  };
}

/** Stable hex string for a receipt prefix — used for keying React lists. */
export function receiptKey(ev: SprawlEvent): string {
  return ev.receipt_causal_prefix
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

/** Room-anchor colour scheme mirroring the Neuromancer arena numbering. */
export const ROOM_COLOR: Record<string, string> = {
  case: '#4db2ff',
  molly: '#e63333',
  wintermute: '#9a1acc',
  three_jane: '#f2f24a',
  angie: '#ff66c4',
  armitage: '#33cc4d',
  corto: '#e68019',
  neuromancer: '#00e5ff',
  loa: '#ffffff',
};
