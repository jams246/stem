function find(rows: number[][]) {
  outer: for (const row of rows) {
    for (const cell of row) {
      if (cell < 0) break outer;
    }
  }
}
