import Link from 'next/link';
import { ALL_RUNS } from '@/lib/runs';

export default function EpisodeSelector() {
  const bySource = ALL_RUNS.reduce<Record<string, typeof ALL_RUNS>>(
    (acc, run) => {
      acc[run.source] = acc[run.source] || [];
      acc[run.source].push(run);
      return acc;
    },
    {}
  );

  return (
    <main
      style={{ padding: 40, maxWidth: 960, margin: '0 auto' }}
      data-testid="episode-selector"
    >
      <h1 style={{ fontSize: 40, letterSpacing: 2, marginBottom: 8 }}>
        Matrix · Sprawl Trilogy
      </h1>
      <p style={{ opacity: 0.7, marginBottom: 24 }}>
        Every run on this page renders live from the same branchless admission
        algebra shipped in the <code>unibit</code> crates. No cliché text
        waterfall — the 64³ globe is the instrument.
      </p>
      <Link
        href="/sprawl"
        data-testid="sprawl-link"
        style={{
          display: 'inline-block',
          padding: '10px 18px',
          marginBottom: 32,
          background: 'linear-gradient(90deg, #4db2ff 0%, #9a1acc 100%)',
          color: '#0c0f18',
          textDecoration: 'none',
          borderRadius: 6,
          fontWeight: 700,
          letterSpacing: 1,
          fontSize: 13,
        }}
      >
        ▶ play the Sprawl MUD — Case to Loa
      </Link>

      {Object.entries(bySource).map(([source, runs]) => (
        <section key={source} style={{ marginBottom: 40 }}>
          <h2
            style={{
              fontSize: 20,
              textTransform: 'uppercase',
              letterSpacing: 3,
              opacity: 0.8,
              borderBottom: '1px solid #222',
              paddingBottom: 8,
            }}
          >
            {source}
          </h2>
          <ul style={{ listStyle: 'none', padding: 0, margin: '16px 0' }}>
            {runs.map((run) => (
              <li key={run.id} style={{ marginBottom: 8 }}>
                <Link
                  href={`/episode/${run.id.toLowerCase()}`}
                  data-testid={`run-link-${run.id.toLowerCase()}`}
                  style={{
                    display: 'flex',
                    gap: 16,
                    padding: 12,
                    background: '#0c0f18',
                    border: '1px solid #1a1d26',
                    borderRadius: 6,
                    color: '#e8e8f0',
                    textDecoration: 'none',
                  }}
                >
                  <span style={{ opacity: 0.5, minWidth: 48 }}>{run.id}</span>
                  <span style={{ flex: 1 }}>{run.title}</span>
                  <span style={{ opacity: 0.5, fontSize: 12 }}>
                    {run.arena}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        </section>
      ))}

      <footer style={{ marginTop: 80, opacity: 0.4, fontSize: 12 }}>
        <p>
          Arenas defined in <code>~/unibit/crates/unibit-e2e</code>. Designed
          in <code>docs/opus/60</code>.
        </p>
      </footer>
    </main>
  );
}
