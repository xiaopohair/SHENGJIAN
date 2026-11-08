export function findLastRemote<T extends { remoteId: string }>(
  remotes: T[],
  lastRemoteId: string,
): T | undefined {
  return remotes.find((remote) => remote.remoteId === lastRemoteId);
}
