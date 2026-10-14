function parse(input: string): number;
function parse(input: number): number;
function parse(input: string | number): number {
  return Number(input);
}
declare function external(value: string): void;
abstract class Shape {
  abstract area(): number;
}
interface Named {
  name(): string;
}
