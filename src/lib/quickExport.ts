// Pure helpers for the one-click export panel.

export type SourceRange = { start: number; end: number };
type Segment = { sourceStart: number; sourceEnd: number };

const MIN_PART_SECONDS = 0.05;

/**
 * What a quick export writes: the kept segments in timeline order, trimmed to the
 * In/Out range when one is set. So "mark In and Out" and "cut out the middle" both
 * do what they look like.
 */
export function quickExportRanges(segments: Segment[], range: SourceRange | null): SourceRange[] {
  return segments
    .map((segment) => ({
      start: range ? Math.max(segment.sourceStart, range.start) : segment.sourceStart,
      end: range ? Math.min(segment.sourceEnd, range.end) : segment.sourceEnd,
    }))
    .filter((part) => part.end - part.start >= MIN_PART_SECONDS)
    .reduce<SourceRange[]>((merged, part) => {
      // Back-to-back parts (a cut with nothing removed) export as one piece: no extra
      // join, and lossless cuts can't repeat frames at a seam.
      const previous = merged[merged.length - 1];
      if (previous && Math.abs(part.start - previous.end) < 0.001) {
        previous.end = part.end;
      } else {
        merged.push({ ...part });
      }
      return merged;
    }, []);
}

export function totalSeconds(ranges: SourceRange[]): number {
  return ranges.reduce((sum, part) => sum + (part.end - part.start), 0);
}

/** `0m04s`, `12m30s`, `1h02m05s` */
export function compactTimestamp(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  const h = Math.floor(whole / 3600);
  const m = Math.floor((whole % 3600) / 60);
  const s = whole % 60;
  const ss = String(s).padStart(2, '0');
  return h > 0 ? `${h}h${String(m).padStart(2, '0')}m${ss}s` : `${m}m${ss}s`;
}

/** `Replay 2026-10-26.mp4` + clip starting at 14 s → `Replay 2026-10-26-0m14s.mp4`, next to the original. */
export function quickExportPath(sourcePath: string, startSeconds: number, extension: string, attempt = 1): string {
  const slash = Math.max(sourcePath.lastIndexOf('\\'), sourcePath.lastIndexOf('/'));
  const folder = slash >= 0 ? sourcePath.slice(0, slash + 1) : '';
  const leaf = sourcePath.slice(slash + 1);
  const stem = leaf.replace(/\.[^.]+$/, '') || 'clip';
  const suffix = attempt > 1 ? ` (${attempt})` : '';
  return `${folder}${stem}-${compactTimestamp(startSeconds)}${suffix}.${extension}`;
}

/** "about 6 s left" from elapsed time and percent done; null until there's enough to go on. */
export function remainingLabel(elapsedSeconds: number, percent: number | null): string | null {
  if (percent === null || percent < 3 || percent >= 100 || elapsedSeconds < 1) {
    return null;
  }
  const remaining = (elapsedSeconds * (100 - percent)) / percent;
  if (remaining < 1.5) {
    return 'almost done';
  }
  if (remaining < 60) {
    return `about ${Math.round(remaining)} s left`;
  }
  return `about ${Math.round(remaining / 60)} min left`;
}
