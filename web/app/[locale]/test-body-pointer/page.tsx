'use client';

import * as React from 'react';

export default function TestBodyPointerPage() {
  const [bodyStyle, setBodyStyle] = React.useState('');
  const [bodyPointerEvents, setBodyPointerEvents] = React.useState('');

  React.useEffect(() => {
    const checkBodyStyle = () => {
      const computed = window.getComputedStyle(document.body);
      setBodyStyle(document.body.getAttribute('style') || 'no inline styles');
      setBodyPointerEvents(computed.pointerEvents);
    };

    // Check immediately
    checkBodyStyle();

    // Check every 500ms
    const interval = setInterval(checkBodyStyle, 500);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="container py-20">
      <h1 className="text-3xl font-bold mb-8">Body Pointer Events Checker</h1>

      <div className="space-y-4 p-6 border rounded-lg">
        <div>
          <h2 className="text-xl font-semibold mb-2">Body Inline Styles:</h2>
          <code className="block p-4 bg-muted rounded">
            {bodyStyle}
          </code>
        </div>

        <div>
          <h2 className="text-xl font-semibold mb-2">Body Computed pointer-events:</h2>
          <code className="block p-4 bg-muted rounded">
            {bodyPointerEvents}
          </code>
        </div>

        {bodyPointerEvents === 'none' && (
          <div className="p-4 bg-destructive/10 border border-destructive rounded">
            <p className="font-bold text-destructive">
              ⚠️ WARNING: Body has pointer-events: none! This will block all interactions.
            </p>
          </div>
        )}

        <div className="text-sm text-muted-foreground">
          <p>This page checks if the body element has pointer-events: none applied.</p>
          <p>Navigate to another page, open tooltips/dialogs, and return here to see if pointer-events persist.</p>
        </div>
      </div>
    </div>
  );
}
