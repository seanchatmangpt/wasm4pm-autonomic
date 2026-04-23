'use client';

import { useEffect } from 'react';

export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    // eslint-disable-next-line no-console
    console.error('matrix-tv global error:', error);
  }, [error]);

  return (
    <html lang="en">
      <body
        data-testid="global-error"
        style={{
          margin: 0,
          minHeight: '100vh',
          background: '#05070d',
          color: '#e8e8f0',
          fontFamily: 'ui-monospace, monospace',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 16,
          padding: 32,
        }}
      >
        <div style={{ fontSize: 10, letterSpacing: 2, opacity: 0.6 }}>
          GLOBAL ERROR
        </div>
        <div style={{ fontSize: 22, color: '#ff9f9f' }}>
          app shell quarantined
        </div>
        <pre
          style={{
            fontSize: 12,
            opacity: 0.7,
            maxWidth: 720,
            overflow: 'auto',
          }}
        >
          {error.message}
        </pre>
        <button
          onClick={reset}
          type="button"
          aria-label="Reboot the app shell"
          style={{
            padding: '8px 16px',
            background: 'rgba(20, 20, 30, 0.8)',
            color: '#e8e8f0',
            border: '1px solid #444',
            borderRadius: 4,
            cursor: 'pointer',
            fontSize: 12,
          }}
        >
          REBOOT
        </button>
      </body>
    </html>
  );
}
