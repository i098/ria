// Turns framegrab's per-frame numbers into one animation measurement.
// diffFirst / diffLast: mean absolute channel difference (0-255) of each frame against the first / last frame.
// progress = diffFirst / (diffFirst + diffLast): 0 at the first frame, 1 at the last, whatever the animation changes.

const median = (xs) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)];

export function measure(frames, markMs) {
  const before = frames.filter((f) => f.ms < markMs);
  const total = median(frames.slice(-5).map((f) => f.diffFirst));
  // Noise: the window's own idle changes (caret, shaders) seen before the trigger.
  const noise = Math.max(0, ...before.map((f) => f.diffFirst));
  const threshold = noise + 0.02 * total;
  const afterMark = frames.filter((f) => f.ms >= markMs);
  const first = afterMark.find((f) => f.diffFirst > threshold);
  if (!first || total <= threshold) throw new Error(`no animation seen (total change ${total.toFixed(3)}, threshold ${threshold.toFixed(3)})`);
  let last = frames.length - 1; // first frame of the trailing run that already shows the end state
  while (last > 0 && frames[last - 1].diffLast <= threshold) last--;
  const end = frames[last];
  const startIdx = frames.indexOf(first) - 1;
  const start = frames[Math.max(0, startIdx)];
  const curve = frames.slice(Math.max(0, startIdx), last + 1).map((f) => ({
    tMs: round(f.ms - start.ms), progress: round(f.diffFirst / Math.max(f.diffFirst + f.diffLast, 1e-9), 4),
  }));
  const at = (p) => {
    const i = curve.findIndex((c) => c.progress >= p);
    if (i <= 0) return curve[0]?.tMs ?? 0;
    const a = curve[i - 1], b = curve[i];
    return round(a.tMs + ((p - a.progress) / (b.progress - a.progress || 1)) * (b.tMs - a.tMs));
  };
  const interval = median(frames.slice(1).map((f, i) => f.ms - frames[i].ms).filter((d) => d < 100));
  return {
    latencyMs: round(start.ms - markMs),
    durationMs: round(end.ms - start.ms),
    t10Ms: at(0.1), t50Ms: at(0.5), t90Ms: at(0.9),
    curve,
    totalChange: round(total, 3), noiseFloor: round(noise, 3), threshold: round(threshold, 3),
    frameIntervalMs: round(interval, 2), framesInAnimation: curve.length,
    startFrame: frames[Math.max(0, startIdx)].file, endFrame: end.file,
  };
}

export const round = (n, d = 1) => Math.round(n * 10 ** d) / 10 ** d;
export const summary = (xs) => ({ min: round(Math.min(...xs)), median: round(median(xs)), max: round(Math.max(...xs)) });
