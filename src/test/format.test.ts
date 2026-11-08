import { describe, expect, it } from "vitest";
import { formatDate } from "@/lib/format";

describe("formatDate", () => {
  it("keeps date-only values on the same calendar day", () => {
    const formatted = formatDate("2026-11-01");
    expect(formatted).toContain("2026");
    expect(formatted).toMatch(/01|1/);
    expect(formatted).not.toContain("Oct");
    expect(formatted).not.toContain("31");
  });
});
