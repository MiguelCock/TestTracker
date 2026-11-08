import { useMutation, useQuery } from "@tanstack/react-query";
import { CheckCircle2, ShieldCheck, XCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import { formatDateTime, truncate } from "@/lib/format";

export function AuditPanel({ engagementId }: { engagementId: string }) {
  const events = useQuery({
    queryKey: ["audit", engagementId],
    queryFn: () => api.listAuditEvents(engagementId, 200),
  });
  const verify = useMutation({ mutationFn: api.verifyAuditChain });

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-white">Audit history</h2>
          <p className="text-xs text-slate-500">
            Append-only, hash-chained events. Any tampering breaks the chain.
          </p>
        </div>
        <Button
          variant="secondary"
          size="sm"
          onClick={() => verify.mutate()}
          disabled={verify.isPending}
        >
          {verify.isPending ? <Spinner /> : <ShieldCheck className="h-3.5 w-3.5" />}
          Verify chain
        </Button>
      </div>

      {verify.data === true && (
        <div className="flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-xs text-emerald-300">
          <CheckCircle2 className="h-4 w-4" /> Audit chain verified.
        </div>
      )}
      {verify.data === false && (
        <div className="flex items-center gap-2 rounded-lg border border-red-500/30 bg-red-500/5 px-3 py-2 text-xs text-red-300">
          <XCircle className="h-4 w-4" /> Audit chain verification failed.
        </div>
      )}
      {verify.isError && <ErrorBlock message={verify.error.message} />}

      {events.isLoading && <LoadingBlock label="Loading audit events…" />}
      {events.isError && <ErrorBlock message={events.error.message} />}

      {events.data && events.data.length === 0 && (
        <EmptyState
          title="No audit events"
          description="Changes to this engagement will appear here."
        />
      )}

      {events.data && events.data.length > 0 && (
        <Card className="overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="border-b border-slate-800 bg-slate-950/40 text-[11px] uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Seq</th>
                  <th className="px-4 py-3 font-medium">Action</th>
                  <th className="px-4 py-3 font-medium">Entity</th>
                  <th className="px-4 py-3 font-medium">Actor</th>
                  <th className="px-4 py-3 font-medium">Time</th>
                  <th className="px-4 py-3 font-medium">Hash</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800">
                {events.data.map((event) => {
                  const metadata = (event.metadata ?? null) as Record<string, unknown> | null;
                  return (
                    <tr key={event.id} className="align-top">
                      <td className="px-4 py-3 text-xs text-slate-500">{event.seq}</td>
                      <td className="px-4 py-3">
                        <Badge tone="accent">{event.action}</Badge>
                        {metadata && Object.keys(metadata).length > 0 && (
                          <details className="mt-1">
                            <summary className="cursor-pointer text-[11px] text-slate-500">
                              metadata
                            </summary>
                            <pre className="mt-1 max-w-xs overflow-x-auto rounded bg-slate-950 p-2 text-[10px] text-slate-400">
                              {JSON.stringify(metadata, null, 2)}
                            </pre>
                          </details>
                        )}
                      </td>
                      <td className="px-4 py-3 text-xs text-slate-400">
                        {event.entityType}
                        {event.entityId ? ` · ${truncate(event.entityId, 12)}` : ""}
                      </td>
                      <td className="px-4 py-3 text-xs text-slate-400">
                        {event.actorId ? truncate(event.actorId, 12) : "—"}
                      </td>
                      <td className="px-4 py-3 text-xs text-slate-400">
                        {formatDateTime(event.createdAt)}
                      </td>
                      <td className="px-4 py-3 font-mono text-[10px] text-slate-600">
                        {truncate(event.hash, 20)}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </Card>
      )}
    </div>
  );
}
