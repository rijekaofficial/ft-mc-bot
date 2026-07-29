// Диапазоны анархий. Бедрок (/anXX) намеренно не включён.
const RANGES: Array<[number, number]> = [
  [101, 114],
  [201, 228],
  [301, 325],
  [501, 516],
  [901, 904],
];

export const ANARCHIES: string[] = RANGES.flatMap(([from, to]) => {
  const out: string[] = [];
  for (let i = from; i <= to; i++) out.push(`an${i}`);
  return out;
});

export const ANARCHY_COUNT = ANARCHIES.length;
