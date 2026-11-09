import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { McpPage } from "@/features/mcp/McpPage";

vi.mock("@/lib/api", () => ({
  api: {
    mcpStatus: vi.fn(async () => ({
      running: true,
      endpoint: "http://127.0.0.1:37621/mcp",
      port: 37621,
      hasToken: true,
      pendingCount: 1,
      startedAt: "2026-01-01T00:00:00Z",
    })),
    mcpListApprovals: vi.fn(async () => [
      {
        approvalId: "01a1bb8f-c71c-70fd-a7d7-95c659b94ff8",
        state: "pending",
        engagementId: "01a1bb8f-a3da-7122-bc5a-ac68a846a236",
        engagementName: "Synthetic MCP Assessment",
        skill: "test_cases",
        targetId: null,
        targetLabel: null,
        payload: { testCases: [{ title: "Synthetic authorization check" }] },
        client: null,
        createdAt: "2026-01-01T00:00:00Z",
        expiresInSeconds: 600,
        result: null,
      },
    ]),
    mcpStart: vi.fn(),
    mcpStop: vi.fn(),
    mcpRevealToken: vi.fn(),
    mcpRegenerateToken: vi.fn(),
    mcpApprove: vi.fn(),
    mcpReject: vi.fn(),
  },
}));

afterEach(cleanup);

describe("McpPage", () => {
  it("shows server status and pending proposal summaries", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { container } = render(
      <QueryClientProvider client={client}>
        <McpPage />
      </QueryClientProvider>,
    );

    expect(await screen.findByText("Synthetic MCP Assessment")).toBeInTheDocument();
    expect(screen.getAllByText("Running").length).toBeGreaterThan(0);
    expect(container.textContent).toContain("Synthetic authorization check");
    expect(screen.getByText("Approve")).toBeInTheDocument();
  });
});
