export function nesting(rows: number[][]) {
  for (const row of rows) {
    if (row.length > 0) {
      for (const cell of row) {
        if (cell > 0) {
          report(cell);
        }
      }
    }
  }
}
