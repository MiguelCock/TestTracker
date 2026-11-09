import { useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { Network } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { api } from "@/lib/api";

export function McpCard() {
  const navigate = useNavigate();
  const status = useQuery({
    queryKey: ["mcp-status"],
    queryFn: api.mcpStatus,
    refetchInterval: 10_000,
    retry: false,
  });
  return (
    <Card className="mt-4">
      <CardHeader
        title="MCP bridge"
        description="Expose local engagement reads and draft proposals to an MCP-capable client."
        action={
          <Badge tone={status.data?.running ? "success" : "neutral"}>
            {status.data?.running ? "Running" : "Stopped"}
          </Badge>
        }
      />
      <CardBody className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-start gap-3 text-xs text-slate-400">
          <Network className="mt-0.5 h-4 w-4 shrink-0 text-emerald-400" />
          <div>
            <p>
              {status.data?.pendingCount ?? 0} pending proposal(s)
              {status.data?.endpoint ? ` · ${status.data.endpoint}` : ""}
            </p>
            <p className="mt-1 text-[11px] text-slate-500">
              Manual start/stop. No direct writes. Stops when the vault locks.
            </p>
          </div>
        </div>
        <Button variant="secondary" size="sm" onClick={() => navigate("/mcp")}>
          Open MCP bridge
        </Button>
      </CardBody>
    </Card>
  );
}
