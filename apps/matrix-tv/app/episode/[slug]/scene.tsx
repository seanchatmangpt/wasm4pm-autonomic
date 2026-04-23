'use client';

import { Canvas } from '@react-three/fiber';
import { useEffect, useMemo, useState } from 'react';
import { Globe } from '@/components/GlobeRenderer';
import { ReceiptRibbon } from '@/components/ReceiptRibbon';
import { VerdictBadge } from '@/components/VerdictBadge';
import { motionTick, MotionResponse } from '@/lib/unibit';
import type { Run } from '@/lib/runs';

export function EpisodeScene({ run }: { run: Run }) {
  const [response, setResponse] = useState<MotionResponse | null>(null);
  const [history, setHistory] = useState<MotionResponse[]>([]);
  const [currentAnnotation, setCurrentAnnotation] = useState<string | null>(
    null
  );

  const initialResponse = useMemo(() => motionTick(run.request), [run]);

  useEffect(() => {
    setResponse(initialResponse);
    setHistory([initialResponse]);
  }, [initialResponse]);

  useEffect(() => {
    const started = Date.now();
    const interval = setInterval(() => {
      const t = (Date.now() - started) / 1000;
      // find the latest annotation whose t <= current elapsed
      const ann = run.annotations
        .filter((a) => a.t <= t)
        .slice(-1)[0];
      setCurrentAnnotation(ann ? ann.text : null);
    }, 100);
    return () => clearInterval(interval);
  }, [run]);

  const rerun = () => {
    const r = motionTick({
      ...run.request,
      instructionId: run.request.instructionId + BigInt(history.length),
    });
    setResponse(r);
    setHistory((prev) => [...prev, r]);
  };

  const tamper = () => {
    // Flip a forbidden bit to force a denial.
    const r = motionTick({
      ...run.request,
      state: run.request.state | 0b0010_0000n,
      instructionId: run.request.instructionId + BigInt(history.length),
    });
    setResponse(r);
    setHistory((prev) => [...prev, r]);
  };

  return (
    <div
      style={{ position: 'relative', height: '100vh' }}
      data-testid="episode-scene"
      data-history-length={history.length}
    >
      <div data-testid="globe-canvas-wrapper" style={{ width: '100%', height: '100%' }}>
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

      {currentAnnotation && (
        <div
          data-testid="annotation"
          style={{
            position: 'absolute',
            bottom: 120,
            left: '50%',
            transform: 'translateX(-50%)',
            padding: '8px 16px',
            background: 'rgba(12, 15, 24, 0.8)',
            border: '1px solid #333',
            borderRadius: 4,
            fontFamily: 'ui-monospace, monospace',
            fontSize: 13,
          }}
        >
          {currentAnnotation}
        </div>
      )}

      <div
        style={{
          position: 'absolute',
          top: 24,
          right: 24,
          marginTop: 80,
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
        }}
      >
        <button
          onClick={rerun}
          style={buttonStyle}
          data-testid="run-button"
          aria-label="Run this motion again with the next instruction id"
          type="button"
        >
          RUN
        </button>
        <button
          onClick={tamper}
          style={{ ...buttonStyle, borderColor: '#e63333' }}
          data-testid="tamper-button"
          aria-label="Tamper with the state to force a denial"
          type="button"
        >
          TAMPER
        </button>
      </div>

      <ReceiptRibbon history={history} />
    </div>
  );
}

const buttonStyle: React.CSSProperties = {
  padding: '8px 16px',
  background: 'rgba(20, 20, 30, 0.8)',
  color: '#e8e8f0',
  border: '1px solid #444',
  borderRadius: 4,
  cursor: 'pointer',
  fontFamily: 'ui-monospace, monospace',
  letterSpacing: 2,
  fontSize: 12,
};
