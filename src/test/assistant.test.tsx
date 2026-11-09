import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AssistantPanel } from "@/features/ai/AssistantPanel";

vi.mock("@/lib/api", () => ({
  api: {
    sessionInfo: vi.fn(async () => ({
      initialized: true,
      unlocked: true,
      profile: { id: "profile-1", displayName: "Synthetic Tester", role: "admin" },
      autoLockSeconds: 900,
    })),
    listAiProviders: vi.fn(async () => [
      {
        id: "provider-1",
        name: "Mock provider",
        kind: "openai_compatible",
        baseUrl: "http://127.0.0.1:18080/v1",
        model: "mock",
        hasKey: false,
        isLocal: true,
        supportsVision: true,
        createdAt: "2026-01-01T00:00:00Z",
      },
    ]),
    getAiPolicy: vi.fn(async () => "local_only"),
    setAiPolicy: vi.fn(),
    listFindings: vi.fn(async () => []),
    listEvidence: vi.fn(async () => []),
    listTestCases: vi.fn(async () => []),
    previewAssistantChat: vi.fn(),
    runAssistantChat: vi.fn(),
    resolveAssistantTool: vi.fn(),
    applyAgentProposal: vi.fn(),
    listEngagementFiles: vi.fn(async () => []),
    addEngagementFile: vi.fn(),
    deleteEngagementFile: vi.fn(),
    listToolPermissions: vi.fn(async () => []),
    revokeToolPermission: vi.fn(),
  },
}));

afterEach(cleanup);

describe("AssistantPanel", () => {
  it("is the engagement landing assistant and shows quick prompts", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={client}>
        <AssistantPanel
          engagementId="engagement-1"
          focus={{ kind: "engagement", id: null }}
          onFocusChange={() => undefined}
        />
      </QueryClientProvider>,
    );

    expect(await screen.findByText("Engagement assistant")).toBeInTheDocument();
    expect(
      await screen.findByText("What should I test next for this engagement?"),
    ).toBeInTheDocument();
    expect(screen.getByText("Vision")).toBeInTheDocument();
  });
});
