'use client';

import { useEffect } from 'react';

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    // eslint-disable-next-line no-console
    console.error('matrix-tv route error:', error);
  }, [error]);

  return (
    <div
      data-testid="route-error"
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
      <div
        style={{
          fontSize: 10,
          letterSpacing: 2,
          opacity: 0.6,
        }}
      >
        ROUTE / ERROR
      </div>
      <div style={{ fontSize: 22, color: '#ff9f9f' }}>
        admission denied at the route boundary
      </div>
      <pre
        style={{
          fontSize: 12,
          opacity: 0.7,
          maxWidth: 720,
          overflow: 'auto',
          background: 'rgba(80, 20, 20, 0.3)',
          border: '1px solid #e63333',
          padding: 12,
          borderRadius: 6,
        }}
      >
        {error.message}
        {error.digest ? `\n\ndigest: ${error.digest}` : ''}
      </pre>
      <button
        onClick={reset}
        type="button"
        aria-label="Retry the failed route"
        style={{
          padding: '8px 16px',
          background: 'rgba(20, 20, 30, 0.8)',
          color: '#e8e8f0',
          border: '1px solid #444',
          borderRadius: 4,
          cursor: 'pointer',
          fontFamily: 'ui-monospace, monospace',
          letterSpacing: 2,
          fontSize: 12,
        }}
      >
        RETRY
      </button>
    </div>
  );
}
