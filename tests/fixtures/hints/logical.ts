export function logical(a: boolean, b: boolean, c: boolean, d: boolean) {
  const first = a && b || c && d;
  const second = a || b && c || d;
  const third = !a && (b || c);
  return first && second || third;
}
