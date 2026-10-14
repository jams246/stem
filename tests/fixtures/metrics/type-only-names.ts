function convert(input: Source): Target {
  const middle: Wrapper<Inner> = input as Middle;
  return middle satisfies Target;
}
