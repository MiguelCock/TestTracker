import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  CheckCircle2,
  Clock,
  Eye,
  EyeOff,
  KeyRound,
  Play,
  RefreshCw,
  Server,
  ShieldCheck,
  Square,
  XCircle,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { PageHeader } from "@/components/layout/PageHeader";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import type { AgentSkill, McpApproval, McpApprovalState } from "@/types";

const skillLabel: Record<AgentSkill, string> = {
  application_summary: "Application summary",
  attack_surface: "Attack surface",
  assumptions: "Assumptions",
  test_cases: "Test cases",
  test_approach: "Test approach",
  finding_draft: "Finding draft",
};

const stateTone: Record<McpApprovalState, "neutral" | "success" | "warning" | "danger"> = {
  pending: "warning",
  approved: "success",
  rejected: "danger",
  expired: "neutral",
};

export function McpPage() {
  const queryClient = useQueryClient();
  const [portInput, setPortInput] = useState("37621");
  const [portDirty, setPortDirty] = useState(false);
  const [token, setToken] = useState<string | null>(null);
  const [tokenVisible, setTokenVisible] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [busyApproval, setBusyApproval] = useState<string | null>(null);

  const status = useQuery({
    queryKey: ["mcp-status"],
    queryFn: api.mcpStatus,
    refetchInterval: 3_000,
  });
  const approvals = useQuery({
    queryKey: ["mcp-approvals"],
    queryFn: api.mcpListApprovals,
    refetchInterval: 3_000,
  });

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["mcp-status"] });
    void queryClient.invalidateQueries({ queryKey: ["mcp-approvals"] });
  };

  const effectivePort = portDirty ? portInput : String(status.data?.port ?? 37621);

  const start = useMutation({
    mutationFn: () => {
      const port = Number(effectivePort);
      if (!Number.isInteger(port) || port < 1024 || port > 65535) {
        throw new Error("Choose a port between 1024 and 65535.");
      }
      return api.mcpStart(port);
    },
    onSuccess: () => {
      setActionError(null);
      setActionMessage("MCP bridge started.");
      setPortDirty(false);
      invalidate();
    },
    onError: (error) => setActionError(error.message),
  });

  const stop = useMutation({
    mutationFn: api.mcpStop,
    onSuccess: () => {
      setActionError(null);
      setActionMessage("MCP bridge stopped.");
      setTokenVisible(false);
      setToken(null);
      invalidate();
    },
    onError: (error) => setActionError(error.message),
  });

  const reveal = useMutation({
    mutationFn: api.mcpRevealToken,
    onSuccess: (value) => {
      setToken(value);
      setTokenVisible(true);
      setActionError(null);
    },
    onError: (error) => setActionError(error.message),
  });

  const regenerate = useMutation({
    mutationFn: api.mcpRegenerateToken,
    onSuccess: (value) => {
      setToken(value);
      setTokenVisible(true);
      setActionError(null);
      setActionMessage("A new MCP token was generated.");
      invalidate();
    },
    onError: (error) => setActionError(error.message),
  });

  const approve = useMutation({
    mutationFn: (approvalId: string) => api.mcpApprove(approvalId),
    onMutate: (approvalId) => {
      setBusyApproval(approvalId);
      setActionError(null);
      setActionMessage(null);
    },
    onSuccess: (result) => {
      setActionMessage(result.message);
      void queryClient.invalidateQueries({ queryKey: ["engagements"] });
      void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
      invalidate();
    },
    onError: (error) => setActionError(error.message),
    onSettled: () => setBusyApproval(null),
  });

  const reject = useMutation({
    mutationFn: (approvalId: string) => api.mcpReject(approvalId),
    onMutate: (approvalId) => {
      setBusyApproval(approvalId);
      setActionError(null);
      setActionMessage(null);
    },
    onSuccess: () => {
      setActionMessage("MCP proposal rejected.");
      invalidate();
    },
    onError: (error) => setActionError(error.message),
    onSettled: () => setBusyApproval(null),
  });

  const running = status.data?.running ?? false;
  const pendingCount = approvals.data?.length ?? 0;
  const endpoint = status.data?.endpoint ?? `http://127.0.0.1:${effectivePort}/mcp`;
  const snippet = JSON.stringify(
    {
      mcpServers: {
        testtracker: {
          type: "http",
          url: endpoint,
          headers: {
            Authorization: `Bearer ${tokenVisible && token ? token : "<token>"}`,
          },
        },
      },
    },
    null,
    2,
  );

  return (
    <div>
      <PageHeader
        title="MCP bridge"
        description="Expose selected engagement tools to a local MCP client. Reads require a bearer token; every draft requires approval here."
        actions={
          <Button
            variant={running ? "danger" : "primary"}
            onClick={() => (running ? stop.mutate() : start.mutate())}
            disabled={status.isLoading || start.isPending || stop.isPending}
          >
            {start.isPending || stop.isPending ? (
              <Spinner />
            ) : running ? (
              <Square className="h-4 w-4" />
            ) : (
              <Play className="h-4 w-4" />
            )}
            {running ? "Stop bridge" : "Start bridge"}
          </Button>
        }
      />

      {status.isLoading && <LoadingBlock label="Loading MCP bridge status…" />}
      {status.isError && <ErrorBlock message={status.error.message} />}

      <div className="space-y-4">
        <Card>
          <CardHeader
            title="Server"
            description="Loopback-only Streamable HTTP endpoint. The bridge stops when the vault locks."
            action={
              <Badge tone={running ? "success" : "neutral"}>
                {running ? "Running" : "Stopped"}
              </Badge>
            }
          />
          <CardBody className="space-y-4">
            <dl className="grid gap-3 text-xs sm:grid-cols-3">
              <div>
                <dt className="text-slate-500">Endpoint</dt>
                <dd className="mt-1 break-all font-mono text-slate-200">{endpoint}</dd>
              </div>
              <div>
                <dt className="text-slate-500">Pending approvals</dt>
                <dd className="mt-1 text-slate-200">{pendingCount}</dd>
              </div>
              <div>
                <dt className="text-slate-500">Started</dt>
                <dd className="mt-1 text-slate-200">
                  {status.data?.startedAt ? new Date(status.data.startedAt).toLocaleString() : "—"}
                </dd>
              </div>
            </dl>

            <div className="flex flex-wrap items-end gap-3">
              <label className="min-w-40 flex-1">
                <span className="mb-1.5 block text-xs font-medium uppercase tracking-wide text-slate-400">
                  Port
                </span>
                <Input
                  type="number"
                  min={1024}
                  max={65535}
                  value={effectivePort}
                  disabled={running}
                  onChange={(event) => {
                    setPortDirty(true);
                    setPortInput(event.target.value);
                  }}
                  className="font-mono"
                />
              </label>
              <Button
                variant="secondary"
                onClick={() => (running ? stop.mutate() : start.mutate())}
                disabled={status.isLoading || start.isPending || stop.isPending}
              >
                {running ? <Square className="h-4 w-4" /> : <Play className="h-4 w-4" />}
                {running ? "Stop" : "Start"}
              </Button>
            </div>

            <div className="rounded-lg border border-slate-800 bg-slate-950/40 p-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-xs font-medium text-slate-300">
                  <KeyRound className="h-4 w-4 text-emerald-400" /> Bearer token
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      if (tokenVisible) {
                        setTokenVisible(false);
                        setToken(null);
                      } else if (token) {
                        setTokenVisible(true);
                      } else {
                        reveal.mutate();
                      }
                    }}
                    disabled={reveal.isPending}
                  >
                    {tokenVisible ? (
                      <EyeOff className="h-3.5 w-3.5" />
                    ) : (
                      <Eye className="h-3.5 w-3.5" />
                    )}
                    {tokenVisible ? "Hide" : "Reveal"}
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      if (
                        window.confirm(
                          "Regenerate the MCP token? Existing MCP clients will stop working until the new token is configured.",
                        )
                      ) {
                        regenerate.mutate();
                      }
                    }}
                    disabled={regenerate.isPending || running}
                    title={running ? "Stop the bridge before regenerating the token." : undefined}
                  >
                    {regenerate.isPending ? <Spinner /> : <RefreshCw className="h-3.5 w-3.5" />}
                    Regenerate
                  </Button>
                </div>
              </div>
              {tokenVisible && token ? (
                <Input readOnly value={token} className="mt-3 font-mono text-xs" />
              ) : (
                <p className="mt-2 text-[11px] text-slate-500">
                  The token is hidden and encrypted at rest. Reveal it only when configuring a local
                  client.
                </p>
              )}
            </div>

            <div className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[11px] text-amber-200/80">
              <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              <span>
                This token grants read and draft access to every engagement. Keep it secret. A
                connected MCP client, including a cloud-backed AI host, can send retrieved data
                off-device. TestTracker only controls local access, not the client.
              </span>
            </div>

            {actionMessage && (
              <div className="flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-xs text-emerald-300">
                <CheckCircle2 className="h-4 w-4" /> {actionMessage}
              </div>
            )}
            {actionError && <ErrorBlock message={actionError} />}
          </CardBody>
        </Card>

        <Card>
          <CardHeader
            title="Pending proposals"
            description="MCP clients can submit drafts. They are applied only when you approve them here."
            action={<Badge tone={pendingCount > 0 ? "warning" : "neutral"}>{pendingCount}</Badge>}
          />
          <CardBody>
            {approvals.isLoading && <LoadingBlock label="Loading proposals…" />}
            {approvals.isError && <ErrorBlock message={approvals.error.message} />}
            {approvals.data && approvals.data.length === 0 && (
              <EmptyState
                title="No pending proposals"
                description="Drafts submitted through the MCP bridge appear here for review."
              />
            )}
            {approvals.data && approvals.data.length > 0 && (
              <div className="space-y-3">
                {approvals.data.map((approval) => (
                  <ApprovalCard
                    key={approval.approvalId}
                    approval={approval}
                    busy={busyApproval === approval.approvalId}
                    onApprove={() => approve.mutate(approval.approvalId)}
                    onReject={() => reject.mutate(approval.approvalId)}
                  />
                ))}
              </div>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader
            title="Client setup"
            description="Use this endpoint and bearer token in an MCP-capable client. HTTP headers are required."
          />
          <CardBody className="space-y-3">
            <pre className="overflow-auto rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs leading-relaxed text-slate-300">
              {snippet}
            </pre>
            <div className="grid gap-2 text-[11px] text-slate-500 sm:grid-cols-2">
              <p className="flex items-start gap-2">
                <Server className="mt-0.5 h-3.5 w-3.5 shrink-0 text-emerald-400" />
                Binds only to 127.0.0.1 and validates Host and Origin. There is no CORS access.
              </p>
              <p className="flex items-start gap-2">
                <ShieldCheck className="mt-0.5 h-3.5 w-3.5 shrink-0 text-emerald-400" />
                Write tools do not exist. Every proposal requires Approve in this desktop app.
              </p>
            </div>
          </CardBody>
        </Card>
      </div>
    </div>
  );
}

function ApprovalCard({
  approval,
  busy,
  onApprove,
  onReject,
}: {
  approval: McpApproval;
  busy: boolean;
  onApprove: () => void;
  onReject: () => void;
}) {
  const expires = Math.max(0, Math.ceil(approval.expiresInSeconds / 60));
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950/40 p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <Badge tone="accent">{skillLabel[approval.skill]}</Badge>
            <Badge tone={stateTone[approval.state]}>{approval.state}</Badge>
            <Badge>{approval.engagementName}</Badge>
            {approval.targetLabel && <Badge tone="info">{approval.targetLabel}</Badge>}
          </div>
          <p className="mt-2 flex items-center gap-1.5 text-[11px] text-slate-500">
            <Clock className="h-3.5 w-3.5" /> Expires in {expires} min
            {approval.client && <span>· client: {approval.client}</span>}
          </p>
        </div>
        <div className="flex gap-2">
          <Button size="sm" variant="secondary" onClick={onReject} disabled={busy}>
            Reject
          </Button>
          <Button size="sm" onClick={onApprove} disabled={busy}>
            {busy ? <Spinner /> : <CheckCircle2 className="h-3.5 w-3.5" />}
            Approve
          </Button>
        </div>
      </div>
      <pre className="mt-3 max-h-64 overflow-auto whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-900/60 p-3 text-[11px] leading-relaxed text-slate-300">
        {JSON.stringify(approval.payload, null, 2)}
      </pre>
      {approval.result && (
        <p className="mt-2 flex items-center gap-1.5 text-[11px] text-emerald-300">
          <CheckCircle2 className="h-3.5 w-3.5" /> {approval.result.message}
        </p>
      )}
      {approval.state === "expired" && (
        <p className="mt-2 flex items-center gap-1.5 text-[11px] text-red-300">
          <XCircle className="h-3.5 w-3.5" /> This proposal expired.
        </p>
      )}
    </div>
  );
}
