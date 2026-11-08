import type { Severity } from "@/types";

export function severityFor(probability: number, impact: number): Severity {
  if (probability < 0 || probability > 9 || impact < 0 || impact > 9) {
    throw new Error("Probability and impact must be between 0 and 9.");
  }
  const p = bucket(probability);
  const i = bucket(impact);
  if (i === 0) return p === 0 ? "informational" : p === 1 ? "low" : "medium";
  if (i === 1) return p === 0 ? "low" : p === 1 ? "medium" : "high";
  return p === 0 ? "medium" : p === 1 ? "high" : "critical";
}

function bucket(value: number): 0 | 1 | 2 {
  if (value <= 2) return 0;
  if (value <= 5) return 1;
  return 2;
}
