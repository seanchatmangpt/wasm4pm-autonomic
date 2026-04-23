import Link from 'next/link';

export default function NotFound() {
  return (
    <div
      data-testid="route-not-found"
      style={{
        minHeight: '100vh',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 16,
        padding: 32,
        fontFamily: 'ui-monospace, monospace',
      }}
    >
      <div style={{ fontSize: 10, letterSpacing: 2, opacity: 0.6 }}>
        404 / NOT FOUND
      </div>
      <div style={{ fontSize: 22, color: '#9ab' }}>
        this run does not exist in the current geometry
      </div>
      <Link
        href="/"
        style={{
          padding: '8px 16px',
          color: '#e8e8f0',
          border: '1px solid #444',
          borderRadius: 4,
          textDecoration: 'none',
          letterSpacing: 2,
          fontSize: 12,
        }}
      >
        RETURN TO RUN SELECTOR
      </Link>
    </div>
  );
}
