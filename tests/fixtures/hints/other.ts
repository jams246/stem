export function other(n: number): number {
  try {
    walk: for (;;) {
      break walk;
    }
  } catch {
    return 0;
  }
  try {
    return other(n - 1) + other(n - 2);
  } catch {
    return -1;
  }
}
