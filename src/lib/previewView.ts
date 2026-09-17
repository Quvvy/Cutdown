export function computeFitScale(
  videoWidth: number,
  videoHeight: number,
  viewportWidth: number,
  viewportHeight: number,
): number {
  if (videoWidth <= 0 || videoHeight <= 0 || viewportWidth <= 0 || viewportHeight <= 0) {
    return 1;
  }

  return Math.min(viewportWidth / videoWidth, viewportHeight / videoHeight);
}

export function shouldAutoFitOnViewportResize(
  userAdjustedView: boolean,
  videoWidth: number,
): boolean {
  return !userAdjustedView && videoWidth > 0;
}
