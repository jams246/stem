export const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

export function slugify(input: string, separator = '-') {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, separator)
    .replace(new RegExp(`^${separator}|${separator}$`, 'g'), '');
}

export const formatters = {
  money(value: number, currency: string) { return `${value.toFixed(2)} ${currency}` },
  percent: (value: number) => `${Math.round(value * 100)}%`,
};
