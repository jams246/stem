export function loops(xs: number[]) {
  let total = 0;
  for (const x of xs) total += x;
  for (const x of xs) total -= x;
  for (let i = 0; i < xs.length; i++) total += i;
  while (total > 100) total -= 100;
  do {
    total += 1;
  } while (total < 10);
  for (const key in xs) total += Number(key);
  return total;
}
