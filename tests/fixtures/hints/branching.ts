export function branching(kind: string) {
  if (kind === "a") {
    return 1;
  } else if (kind === "b") {
    return 2;
  } else if (kind === "c") {
    return 3;
  } else if (kind === "d") {
    return 4;
  } else {
    return kind === "e" ? 5 : 6;
  }
}
