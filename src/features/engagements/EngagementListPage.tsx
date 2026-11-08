import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { Plus, Search, Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { EngagementStatusBadge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Select, SelectItem } from "@/components/ui/select";
import { PageHeader } from "@/components/layout/PageHeader";
import { EmptyState, ErrorBlock, LoadingBlock } from "@/components/ui/states";
import { api } from "@/lib/api";
import { formatDate } from "@/lib/format";
import { EngagementDialog } from "@/features/engagements/EngagementDialog";
import { ImportBundleDialog } from "@/features/bundles/ImportBundleDialog";
import type { EngagementStatus } from "@/types";

export function EngagementListPage() {
  const [createOpen, setCreateOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState<EngagementStatus | "all">("all");
  const engagements = useQuery({ queryKey: ["engagements"], queryFn: api.listEngagements });

  const filtered = useMemo(() => {
    const data = engagements.data ?? [];
    const query = search.trim().toLowerCase();
    return data.filter((engagement) => {
      const matchesStatus = status === "all" || engagement.status === status;
      const matchesSearch =
        !query ||
        engagement.name.toLowerCase().includes(query) ||
        engagement.clientCode.toLowerCase().includes(query);
      return matchesStatus && matchesSearch;
    });
  }, [engagements.data, search, status]);

  return (
    <div>
      <PageHeader
        title="Engagements"
        description="Scope, execute, and review penetration-testing engagements."
        actions={
          <>
            <Button variant="secondary" onClick={() => setImportOpen(true)}>
              <Upload className="h-4 w-4" /> Import bundle
            </Button>
            <Button onClick={() => setCreateOpen(true)}>
              <Plus className="h-4 w-4" /> New engagement
            </Button>
          </>
        }
      />

      <div className="mb-4 flex flex-wrap items-center gap-3">
        <div className="relative flex-1 sm:max-w-sm">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search by name or client code"
            className="pl-9"
          />
        </div>
        <Select
          value={status}
          onValueChange={(value) => setStatus(value as EngagementStatus | "all")}
          className="w-44"
        >
          <SelectItem value="all">All statuses</SelectItem>
          <SelectItem value="draft">Draft</SelectItem>
          <SelectItem value="active">Active</SelectItem>
          <SelectItem value="in_review">In review</SelectItem>
          <SelectItem value="completed">Completed</SelectItem>
          <SelectItem value="archived">Archived</SelectItem>
        </Select>
      </div>

      {engagements.isLoading && <LoadingBlock label="Loading engagements…" />}
      {engagements.isError && <ErrorBlock message={engagements.error.message} />}

      {engagements.data && filtered.length === 0 && (
        <EmptyState
          title={engagements.data.length === 0 ? "No engagements yet" : "No matches"}
          description={
            engagements.data.length === 0
              ? "Create an engagement to define scope, select test cases, and track findings."
              : "Try a different search term or status filter."
          }
          action={
            engagements.data.length === 0 ? (
              <Button size="sm" onClick={() => setCreateOpen(true)}>
                <Plus className="h-3.5 w-3.5" /> New engagement
              </Button>
            ) : undefined
          }
        />
      )}

      {filtered.length > 0 && (
        <Card className="overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="border-b border-slate-800 bg-slate-950/40 text-[11px] uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-5 py-3 font-medium">Engagement</th>
                  <th className="px-5 py-3 font-medium">Status</th>
                  <th className="px-5 py-3 font-medium">Dates</th>
                  <th className="px-5 py-3 text-right font-medium">Tests</th>
                  <th className="px-5 py-3 text-right font-medium">Open findings</th>
                  <th className="px-5 py-3 text-right font-medium">Updated</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800">
                {filtered.map((engagement) => (
                  <tr key={engagement.id} className="transition-colors hover:bg-slate-800/30">
                    <td className="px-5 py-3">
                      <Link to={`/engagements/${engagement.id}`} className="group block">
                        <p className="font-medium text-slate-100 group-hover:text-emerald-300">
                          {engagement.name}
                        </p>
                        <p className="mt-0.5 text-xs text-slate-500">
                          {engagement.clientCode || "No client code"}
                        </p>
                      </Link>
                    </td>
                    <td className="px-5 py-3">
                      <EngagementStatusBadge status={engagement.status} />
                    </td>
                    <td className="px-5 py-3 text-xs text-slate-400">
                      {formatDate(engagement.startDate)} – {formatDate(engagement.endDate)}
                    </td>
                    <td className="px-5 py-3 text-right text-xs text-slate-300">
                      {engagement.testCaseCount}
                    </td>
                    <td className="px-5 py-3 text-right text-xs">
                      <span
                        className={
                          engagement.openFindingCount > 0
                            ? "font-medium text-red-300"
                            : "text-slate-500"
                        }
                      >
                        {engagement.openFindingCount}
                      </span>
                      <span className="text-slate-600"> / {engagement.findingCount}</span>
                    </td>
                    <td className="px-5 py-3 text-right text-xs text-slate-500">
                      {formatDate(engagement.updatedAt)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      )}

      <EngagementDialog open={createOpen} onOpenChange={setCreateOpen} />
      {importOpen && <ImportBundleDialog open={importOpen} onOpenChange={setImportOpen} />}
    </div>
  );
}
