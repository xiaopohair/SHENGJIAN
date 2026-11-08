export function voiceLevelPercent(dbfs: number): number {
  return Math.max(0, Math.min(100, ((dbfs + 60) / 60) * 100));
}
