type LeaveGuard = () => boolean | Promise<boolean>;

const guards = new Set<LeaveGuard>();

export function registerLeaveGuard(guard: LeaveGuard): () => void {
  guards.add(guard);
  return () => {
    guards.delete(guard);
  };
}

export async function canLeavePage(): Promise<boolean> {
  for (const guard of [...guards]) {
    if (!(await guard())) return false;
  }
  return true;
}
