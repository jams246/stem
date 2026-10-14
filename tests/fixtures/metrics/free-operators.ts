async function load(a: { b?: number } | undefined, c: number, x: Promise<void>) {
  const value = a?.b ?? c;
  await x;
  return value;
}
