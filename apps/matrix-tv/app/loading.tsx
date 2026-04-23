export default function Loading() {
  return (
    <div
      data-testid="route-loading"
      style={{
        minHeight: '100vh',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 16,
        fontFamily: 'ui-monospace, monospace',
        color: '#9ab',
      }}
    >
      <div
        style={{
          width: 48,
          height: 48,
          borderRadius: '50%',
          border: '2px solid #1a1d26',
          borderTopColor: '#4db2ff',
          animation: 'spin 0.8s linear infinite',
        }}
        aria-label="loading"
        role="status"
      />
      <div style={{ fontSize: 10, letterSpacing: 2, opacity: 0.6 }}>
        COMPILING MOTION
      </div>
      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}
