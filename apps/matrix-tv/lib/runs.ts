/**
 * The 40 runs from the Sprawl trilogy, mapped to unibit arenas
 * (see docs/opus/60). Table-driven with hand-crafted overrides for
 * the 11 runs that carry bespoke camera choreography and annotations.
 */

import { CANONICAL_FIELDS, MotionRequest } from './unibit';

export interface CameraKeyframe {
  pos: [number, number, number];
  look: [number, number, number];
  t: number;
}

export interface Annotation {
  t: number;
  text: string;
}

export interface Run {
  id: string;
  title: string;
  source: 'Neuromancer' | 'Count Zero' | 'Mona Lisa Overdrive' | 'unibit';
  arena: string;
  request: MotionRequest;
  expected: {
    admitted: boolean;
    description: string;
  };
  camera: CameraKeyframe[];
  annotations: Annotation[];
}

// =============================================================================
// Row format — compact table driving most of the corpus
// =============================================================================

type Source = 'Neuromancer' | 'Count Zero' | 'Mona Lisa Overdrive';
type CameraStyle = 'orbit' | 'descend' | 'push-in' | 'static' | 'pan';

interface SprawlRow {
  id: string;
  title: string;
  source: Source;
  arena: string;
  /** Marking; defaults to all-required-bits-set if omitted. */
  state?: bigint;
  /** Instruction id; defaults to a deterministic hash of the row id. */
  instructionId?: bigint;
  /** Expected outcome; defaults to `!( state & 0xF0 )`. */
  admitted?: boolean;
  description: string;
  /** Single-line annotation overlaid at t = 1.0 s. */
  highlight?: string;
  /** Camera pattern; defaults to 'orbit' for admits, 'push-in' for denials. */
  cameraStyle?: CameraStyle;
}

function defaultInstructionId(id: string): bigint {
  // Deterministic low-entropy hash: accumulate bytes.
  let h = 0x0100n;
  for (const c of id) {
    h = (h * 31n + BigInt(c.charCodeAt(0))) & 0xFFFFFFFFn;
  }
  return h;
}

function defaultState(): bigint {
  return 0b0000_1111n; // all required, no forbidden
}

function defaultAdmittedFor(state: bigint): boolean {
  return (state & 0b1111_0000n) === 0n && (state & 0b1111n) === 0b1111n;
}

function defaultCamera(style: CameraStyle, admitted: boolean): CameraKeyframe[] {
  const r = admitted ? 6 : 5; // tighter on denials
  switch (style) {
    case 'orbit':
      return [
        { pos: [0, 0, r + 4], look: [0, 0, 0], t: 0 },
        { pos: [3, 2, r - 2], look: [0, 0, 0], t: 3 },
      ];
    case 'descend':
      return [
        { pos: [0, 0, r + 4], look: [0, 0, 0], t: 0 },
        { pos: [0, 0, 2], look: [0, 0, 0], t: 6 },
      ];
    case 'push-in':
      return [
        { pos: [0, 0, r + 2], look: [0, 0, 0], t: 0 },
        { pos: [0, 0, 3], look: [0, 0, 0], t: 4 },
      ];
    case 'pan':
      return [
        { pos: [-4, 0, r], look: [0, 0, 0], t: 0 },
        { pos: [4, 0, r], look: [0, 0, 0], t: 4 },
      ];
    case 'static':
    default:
      return [{ pos: [0, 0, r + 1], look: [0, 0, 0], t: 0 }];
  }
}

function defaultAnnotations(row: SprawlRow, admitted: boolean): Annotation[] {
  const text = row.highlight ?? (admitted ? 'admission sealed' : 'denial emitted');
  return [{ t: 1.0, text }];
}

function buildRun(row: SprawlRow): Run {
  const state = row.state ?? defaultState();
  const admitted = row.admitted ?? defaultAdmittedFor(state);
  const style: CameraStyle =
    row.cameraStyle ?? (admitted ? 'orbit' : 'push-in');
  return {
    id: row.id,
    title: row.title,
    source: row.source,
    arena: row.arena,
    request: {
      state,
      fields: CANONICAL_FIELDS,
      instructionId: row.instructionId ?? defaultInstructionId(row.id),
    },
    expected: { admitted, description: row.description },
    camera: defaultCamera(style, admitted),
    annotations: defaultAnnotations(row, admitted),
  };
}

// =============================================================================
// The table — 30 rows spanning Neuromancer, Count Zero, Mona Lisa Overdrive
// =============================================================================

const SPRAWL_ROWS: SprawlRow[] = [
  // ---- Neuromancer (10) ----
  { id: 'N1',  title: "Case's first jack-in at the Chiba clinic",   source: 'Neuromancer', arena: 'arena_15_l1_region',       description: 'HotRegion materialises; L1 position receipt seals', highlight: 'Pin<Box<L1Region>> allocated' },
  { id: 'N2',  title: 'Dixie Flatline construct replay',             source: 'Neuromancer', arena: 'arena_03_snapshot_nesting', description: 'Nested Snapshot verifies deep', highlight: 'BLAKE3 seals re-derive', cameraStyle: 'descend' },
  { id: 'N3',  title: 'First ICE encounter (Sense/Net)',              source: 'Neuromancer', arena: 'arena_17_hot_kernels',     state: 0b0001_1111n, admitted: false, description: 'Law lane fires a red flare', highlight: 'Law mask matches — denial' },
  { id: 'N4',  title: 'Sushi bar meet; Wintermute phone',             source: 'Neuromancer', arena: 'arena_12_powl_lockstep',   description: 'Semantic intent compiles to MotionPacket', highlight: 'compile(HPowl) → packet' },
  { id: 'N5',  title: 'Villa Straylight approach',                    source: 'Neuromancer', arena: 'arena_15_l1_region',       description: 'HotRegion rotates into view; layout asserts flash', highlight: 'align(64) verified' },
  { id: 'N6',  title: "Ratz's bar, the scorched trodes",              source: 'Neuromancer', arena: 'arena_04_watchdog_liveness', description: 'Watchdog ring dims; recovers via tick', highlight: 'Watchdog countdown visible' },
  { id: 'N7',  title: 'Turing police raid on the Villa',              source: 'Neuromancer', arena: 'arena_07_chain_replay',    state: 0b1111_0000n, admitted: false, description: 'Every lane denies; quarantine halo', highlight: 'all 8 lanes fire', cameraStyle: 'pan' },
  { id: 'N8',  title: 'Wintermute + Neuromancer fusion',              source: 'Neuromancer', arena: 'arena_03_snapshot_nesting', description: 'Two Snapshots merge; outer seal re-derives', highlight: 'depth += 1', cameraStyle: 'descend' },
  { id: 'N9',  title: 'Molly’s corporate run on Sense/Net',           source: 'Neuromancer', arena: 'arena_13_worker_pool',     description: '1000 geodesics; some red, some green', highlight: 'pool dispatch 1000×' },
  { id: 'N10', title: "The flatline's last laugh",                    source: 'Neuromancer', arena: 'arena_22_snapshot_seal',   description: 'seal difference becomes visible', highlight: 'leaf vs wrapped seal mismatch' },

  // ---- Count Zero (10) ----
  { id: 'CZ1',  title: "Bobby Newmark's first run, near-flatline",    source: 'Count Zero', arena: 'arena_04_watchdog_liveness', description: 'Watchdog almost zeroes; tick saves the run', highlight: 'counter: 3 ... 2 ... 1 ... tick' },
  { id: 'CZ2',  title: 'The Virek contract (biomed vat)',              source: 'Count Zero', arena: 'arena_34_resident_long_run', description: 'Snapshot re-seals every cycle; depth invariant', highlight: 'resident tick' },
  { id: 'CZ3',  title: 'Legba rides Bobby',                             source: 'Count Zero', arena: 'arena_31_commandeering_override', description: 'Prereq lane commandeers; color saturates', highlight: 'LaneMode::Commandeering' },
  { id: 'CZ4',  title: "The Boxmaker's assemblages",                    source: 'Count Zero', arena: 'arena_35_orphan_assembler',  state: 0b1111_0000n, admitted: false, description: 'Fragments scatter; Boxmaker folds them into a Snapshot', highlight: 'orphan pool → Snapshot' },
  { id: 'CZ5',  title: "Turner's extraction",                           source: 'Count Zero', arena: 'arena_36_endpoint_transfer', description: 'Snapshot travels between two endpoints', highlight: 'transfer() across endpoints' },
  { id: 'CZ6',  title: 'Marly walks the Cornell boxes',                 source: 'Count Zero', arena: 'arena_27_multi_motion_replay', description: 'Receipt ribbon animates; chain verifies', highlight: 'chain replay walks the tape' },
  { id: 'CZ7',  title: 'Gentleman Loser bar — fixer trades',            source: 'Count Zero', arena: 'arena_37_broker_channel',    description: 'Fragments flow through SpscRing broker', highlight: 'broker preserves FIFO' },
  { id: 'CZ8',  title: 'Biosoft slotted into skull',                    source: 'Count Zero', arena: 'arena_38_signed_snapshot',   description: 'External SignedSnapshot docks onto active region', highlight: 'blake3 signature verified' },
  { id: 'CZ9',  title: "The Finn's shop — one-of-a-kinds",              source: 'Count Zero', arena: 'arena_05_ring_fragments',    description: '64 fragment boxes pulse in sequence', highlight: 'u128 fragments cycle' },
  { id: 'CZ10', title: 'Void past the sprawl — orbital silence',        source: 'Count Zero', arena: 'arena_39_residence_ladder',  description: 'Region fades Reg → L1 → L2 → L3 → DRAM', highlight: 'residence monotone increases', cameraStyle: 'pan' },

  // ---- Mona Lisa Overdrive (10) ----
  { id: 'MLO1',  title: "Mona's first simstim session",                 source: 'Mona Lisa Overdrive', arena: 'arena_40_simstim_rehearse', description: 'Scratchpad-only run; no truth mutation', highlight: 'rehearse(candidate)' },
  { id: 'MLO2',  title: 'The Aleph — Straylight recreated',             source: 'Mona Lisa Overdrive', arena: 'arena_03_snapshot_nesting', description: 'Deep-nested Snapshot to MAX_DEPTH = 16', highlight: 'innermost Snapshot reached', cameraStyle: 'descend' },
  { id: 'MLO3',  title: 'Kumiko in London',                              source: 'Mona Lisa Overdrive', arena: 'arena_41_envelope_descend', description: 'Camera descends Aleph.inner with breadcrumbs', highlight: 'envelope walk', cameraStyle: 'descend' },
  { id: 'MLO4',  title: "Angie's dustings (cortical slow-drip)",          source: 'Mona Lisa Overdrive', arena: 'arena_42_long_interval_tick', description: 'Watchdog ticks every N cycles, not every 1', highlight: 'slow-drip sustains life' },
  { id: 'MLO5',  title: "Gentry's shape theory",                          source: 'Mona Lisa Overdrive', arena: 'arena_43_geometry_full_jtbd', description: 'All 8 geometry JTBDs exercised in one scene', highlight: 'JTBD 1..8 cascade' },
  { id: 'MLO6',  title: "The Count's return",                             source: 'Mona Lisa Overdrive', arena: 'arena_08_publish_readiness', description: 'Two runs side-by-side; receipts match bit-for-bit', highlight: 'deterministic replay — identical seals' },
  { id: 'MLO7',  title: 'Tessier-Ashpool voidsmen',                       source: 'Mona Lisa Overdrive', arena: 'arena_23_watchdog_isolation', description: '8-core ring where shards go dark independently', highlight: 'shard isolation holds' },
  { id: 'MLO8',  title: 'Bobby in the Aleph',                             source: 'Mona Lisa Overdrive', arena: 'arena_32_full_workflow',    description: '3-activity workflow rendered as Snapshot stack', highlight: 'Receive → Validate → Release' },
  { id: 'MLO9',  title: "Slick Henry's Judge",                            source: 'Mona Lisa Overdrive', arena: 'arena_44_construct_assembly', description: 'Boxmaker + Turner + Finn + Marly composed', highlight: 'full pipeline composition', cameraStyle: 'pan' },
  { id: 'MLO10', title: "The Jammer's last run",                          source: 'Mona Lisa Overdrive', arena: 'arena_33_publish_checklist', description: 'Every indicator greens; manifesto audit passes', highlight: 'pinned · branchless · typed · receipted · narrow' },
];

// =============================================================================
// Hand-crafted overrides — preserve the 11 original runs with bespoke cameras
// =============================================================================

const HAND_CRAFTED: Record<string, Run> = {
  N1: {
    id: 'N1',
    title: "Case's first jack-in at the Chiba clinic",
    source: 'Neuromancer',
    arena: 'arena_15_l1_region',
    request: { state: 0b0000_1111n, fields: CANONICAL_FIELDS, instructionId: 0x0101n },
    expected: { admitted: true, description: 'HotRegion materialises; L1 position receipt seals' },
    camera: [
      { pos: [0, 0, 10], look: [0, 0, 0], t: 0 },
      { pos: [3, 2, 4], look: [0, 0, 0], t: 3 },
    ],
    annotations: [
      { t: 0.5, text: 'Pin<Box<L1Region>> allocated' },
      { t: 2.0, text: 'mlock succeeds; position validated' },
    ],
  },
  N2: {
    id: 'N2',
    title: 'Dixie Flatline construct replay',
    source: 'Neuromancer',
    arena: 'arena_03_snapshot_nesting',
    request: { state: 0b0000_1111n, fields: CANONICAL_FIELDS, instructionId: 0x0102n },
    expected: { admitted: true, description: 'Nested Snapshot verifies deep' },
    camera: [
      { pos: [0, 0, 8], look: [0, 0, 0], t: 0 },
      { pos: [0, 5, 2], look: [0, 0, 0], t: 5 },
    ],
    annotations: [
      { t: 1.0, text: 'Snapshot inner loads' },
      { t: 3.0, text: 'BLAKE3 seals re-derive' },
    ],
  },
  N3: {
    id: 'N3',
    title: 'First ICE encounter (Sense/Net)',
    source: 'Neuromancer',
    arena: 'arena_17_hot_kernels',
    request: { state: 0b0001_1111n, fields: CANONICAL_FIELDS, instructionId: 0x0103n },
    expected: { admitted: false, description: 'Law lane fires a red flare' },
    camera: [
      { pos: [2, 0, 5], look: [0, 0, 0], t: 0 },
      { pos: [-2, 1, 3], look: [0, 0, 0], t: 4 },
    ],
    annotations: [
      { t: 1.5, text: 'Law mask matches — denial' },
      { t: 2.5, text: 'Fragment emitted; Watchdog ticked' },
    ],
  },
  N6: {
    id: 'N6',
    title: "Ratz's bar, the scorched trodes",
    source: 'Neuromancer',
    arena: 'arena_04_watchdog_liveness',
    request: { state: 0b0000_1111n, fields: CANONICAL_FIELDS, instructionId: 0x0106n },
    expected: { admitted: true, description: 'Watchdog ring dims; recovers via tick' },
    camera: [{ pos: [0, 0, 6], look: [0, 0, 0], t: 0 }],
    annotations: [{ t: 1.0, text: 'Watchdog countdown visible' }],
  },
  N7: {
    id: 'N7',
    title: 'Turing police raid on the Villa',
    source: 'Neuromancer',
    arena: 'arena_07_chain_replay',
    request: { state: 0b1111_0000n, fields: CANONICAL_FIELDS, instructionId: 0x0107n },
    expected: { admitted: false, description: 'Every lane denies; quarantine halo' },
    camera: [{ pos: [0, 0, 7], look: [0, 0, 0], t: 0 }],
    annotations: [{ t: 1.0, text: 'All 8 lanes fire simultaneously' }],
  },
  CZ1: {
    id: 'CZ1',
    title: "Bobby Newmark's first run, near-flatline",
    source: 'Count Zero',
    arena: 'arena_04_watchdog_liveness',
    request: { state: 0b0000_1111n, fields: CANONICAL_FIELDS, instructionId: 0x0201n },
    expected: { admitted: true, description: 'Watchdog almost zeroes; tick saves the run' },
    camera: [{ pos: [0, 0, 6], look: [0, 0, 0], t: 0 }],
    annotations: [{ t: 1.5, text: 'Counter: 3... 2... 1... tick.' }],
  },
  CZ3: {
    id: 'CZ3',
    title: 'Legba rides Bobby',
    source: 'Count Zero',
    arena: 'arena_31_commandeering_override',
    request: { state: 0b0000_1111n, fields: CANONICAL_FIELDS, instructionId: 0x0203n },
    expected: { admitted: true, description: 'Prereq lane commandeers; color saturates' },
    camera: [{ pos: [0, 0, 8], look: [0, 0, 0], t: 0 }],
    annotations: [{ t: 1.0, text: 'Prereq lane mode: Commandeering' }],
  },
  CZ4: {
    id: 'CZ4',
    title: "The Boxmaker's assemblages",
    source: 'Count Zero',
    arena: 'arena_35_orphan_assembler',
    request: { state: 0b1111_0000n, fields: CANONICAL_FIELDS, instructionId: 0x0204n },
    expected: { admitted: false, description: 'Fragments scatter; Boxmaker folds them into a Snapshot' },
    camera: [
      { pos: [0, 0, 10], look: [0, 0, 0], t: 0 },
      { pos: [4, 0, 3], look: [0, 0, 0], t: 4 },
    ],
    annotations: [
      { t: 1.0, text: 'Fragments dropped into the orphan pool' },
      { t: 3.0, text: 'Snapshot seal computes' },
    ],
  },
  MLO2: {
    id: 'MLO2',
    title: 'The Aleph — Straylight recreated',
    source: 'Mona Lisa Overdrive',
    arena: 'arena_03_snapshot_nesting',
    request: { state: 0b0000_1111n, fields: CANONICAL_FIELDS, instructionId: 0x0302n },
    expected: { admitted: true, description: 'Deep-nested Snapshot to MAX_DEPTH = 16' },
    camera: [
      { pos: [0, 0, 10], look: [0, 0, 0], t: 0 },
      { pos: [0, 0, 2], look: [0, 0, 0], t: 8 },
    ],
    annotations: [
      { t: 2.0, text: 'Camera descends Aleph.inner' },
      { t: 6.0, text: 'Innermost Snapshot reached' },
    ],
  },
  MLO6: {
    id: 'MLO6',
    title: "The Count's return",
    source: 'Mona Lisa Overdrive',
    arena: 'arena_08_publish_readiness',
    request: { state: 0b0000_1111n, fields: CANONICAL_FIELDS, instructionId: 0x0306n },
    expected: { admitted: true, description: 'Two runs side-by-side; receipts match bit-for-bit' },
    camera: [{ pos: [0, 0, 8], look: [0, 0, 0], t: 0 }],
    annotations: [{ t: 1.0, text: 'Deterministic replay — identical seals' }],
  },
  MLO10: {
    id: 'MLO10',
    title: "The Jammer's last run",
    source: 'Mona Lisa Overdrive',
    arena: 'arena_33_publish_checklist',
    request: { state: 0b0000_1111n, fields: CANONICAL_FIELDS, instructionId: 0x030an },
    expected: { admitted: true, description: 'Every indicator greens; five-word manifesto audit passes' },
    camera: [{ pos: [0, 0, 8], look: [0, 0, 0], t: 0 }],
    annotations: [
      { t: 0.5, text: 'pinned ✓' },
      { t: 1.0, text: 'branchless ✓' },
      { t: 1.5, text: 'typed ✓' },
      { t: 2.0, text: 'receipted ✓' },
      { t: 2.5, text: 'narrow ✓' },
    ],
  },
};

// =============================================================================
// Assembly
// =============================================================================

export const ALL_RUNS: Run[] = SPRAWL_ROWS.map(
  (row) => HAND_CRAFTED[row.id] ?? buildRun(row)
);

export function runBySlug(slug: string): Run | undefined {
  return ALL_RUNS.find((r) => r.id.toLowerCase() === slug.toLowerCase());
}
