// Disable user zoom in the webview. The window is a fixed-size, non-resizable
// chrome-less panel, so zooming only ever corrupts the layout. Covers every
// zoom vector on WebKitGTK: Ctrl+wheel (pinch arrives as ctrl+wheel too),
// Ctrl +/-/=/0, and Safari-style gesture events (no-op elsewhere).
const stop = (e: Event): void => {
  e.preventDefault();
  e.stopPropagation();
};

window.addEventListener(
  'wheel',
  (e: WheelEvent) => {
    if (e.ctrlKey) stop(e);
  },
  { passive: false, capture: true },
);

window.addEventListener(
  'keydown',
  (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && ['+', '-', '=', '0'].includes(e.key)) stop(e);
  },
  { passive: false, capture: true },
);

for (const evt of ['gesturestart', 'gesturechange', 'gestureend']) {
  window.addEventListener(evt, stop, { passive: false, capture: true });
}

export {};
