import { describe, expect, it } from "vitest";
import { severityFor } from "@/lib/severity";

describe("severityFor", () => {
  it("matches the approved probability × impact matrix", () => {
    expect(severityFor(0, 0)).toBe("informational");
    expect(severityFor(2, 2)).toBe("informational");
    expect(severityFor(3, 0)).toBe("low");
    expect(severityFor(5, 2)).toBe("low");
    expect(severityFor(6, 0)).toBe("medium");
    expect(severityFor(9, 2)).toBe("medium");
    expect(severityFor(0, 3)).toBe("low");
    expect(severityFor(5, 5)).toBe("medium");
    expect(severityFor(6, 3)).toBe("high");
    expect(severityFor(9, 5)).toBe("high");
    expect(severityFor(0, 6)).toBe("medium");
    expect(severityFor(5, 9)).toBe("high");
    expect(severityFor(6, 6)).toBe("critical");
    expect(severityFor(9, 9)).toBe("critical");
  });

  it("rejects out-of-range values", () => {
    expect(() => severityFor(10, 0)).toThrow();
    expect(() => severityFor(0, -1)).toThrow();
  });
});
