import { useState } from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { Activity, ArrowRight, CheckCircle2, FolderKanban, Plus, ShieldAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { EngagementStatusBadge, SeverityBadge } from "@/components/ui/badge";
import { PageHeader } from "@/components/layout/PageHeader";
import { EmptyState, ErrorBlock, LoadingBlock } from "@/components/ui/states";
import { api } from "@/lib/api";
import { formatDate } from "@/lib/format";
import { EngagementDialog } from "@/features/engagements/EngagementDialog";
import type { Severity } from "@/types";

export function DashboardPage() {
  const [createOpen, setCreateOpen] = useState(false);
  const metrics = useQuery({ queryKey: ["dashboard"], queryFn: api.dashboardMetrics });
  const engagements = useQuery({ queryKey: ["engagements"], queryFn: api.listEngagements });

  const progress = metrics.data?.testProgress;
  const executed = progress ? progress.done : 0;
  const applicable = progress ? progress.total - progress.notApplicable : 0;
  const completion = applicable > 0 ? Math.round((executed / applicable) * 100) : 0;
  const highCritical =
    (metrics.data?.findingsBySeverity.high ?? 0) + (metrics.data?.findingsBySeverity.critical ?? 0);

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description="Portfolio health, open findings, and testing progress across engagements."
        actions={
          <Button onClick={() => setCreateOpen(true)}>
            <Plus className="h-4 w-4" /> New engagement
          </Button>
        }
      />

      {metrics.isError && <ErrorBlock message={metrics.error.message} />}
      {metrics.isLoading && <LoadingBlock label="Loading metrics…" />}

      {metrics.data && (
        <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
          <StatCard
            icon={<FolderKanban className="h-4 w-4" />}
            label="Active engagements"
            value={String(metrics.data.activeEngagements)}
            detail={`${metrics.data.totalEngagements} total`}
          />
          <StatCard
            icon={<ShieldAlert className="h-4 w-4" />}
            label="Open findings"
            value={String(metrics.data.openFindings)}
            detail={`${highCritical} high or critical`}
            tone={highCritical > 0 ? "danger" : "default"}
          />
          <StatCard
            icon={<Activity className="h-4 w-4" />}
            label="Test completion"
            value={`${completion}%`}
            detail={`${executed} of ${Math.max(applicable, 0)} applicable tests done`}
          />
          <StatCard
            icon={<CheckCircle2 className="h-4 w-4" />}
            label="Blocked tests"
            value={String(progress?.blocked ?? 0)}
            detail={`${progress?.inProgress ?? 0} in progress`}
            tone={(progress?.blocked ?? 0) > 0 ? "warning" : "default"}
          />
        </div>
      )}

      <div className="mt-6 grid gap-6 xl:grid-cols-[1.6fr_1fr]">
        <Card>
          <CardHeader
            title="Recent engagements"
            description="Most recently updated assessments."
            action={
              <Link
                to="/engagements"
                className="inline-flex items-center gap-1 text-xs text-emerald-400 hover:text-emerald-300"
              >
                View all <ArrowRight className="h-3.5 w-3.5" />
              </Link>
            }
          />
          <CardBody className="p-0">
            {engagements.isLoading && <LoadingBlock label="Loading engagements…" />}
            {engagements.isError && (
              <div className="p-5">
                <ErrorBlock message={engagements.error.message} />
              </div>
            )}
            {engagements.data && engagements.data.length === 0 && (
              <div className="p-5">
                <EmptyState
                  title="No engagements yet"
                  description="Create your first engagement to start scoping, tracking test cases, and recording findings."
                  action={
                    <Button size="sm" onClick={() => setCreateOpen(true)}>
                      <Plus className="h-3.5 w-3.5" /> New engagement
                    </Button>
                  }
                />
              </div>
            )}
            {engagements.data && engagements.data.length > 0 && (
              <ul className="divide-y divide-slate-800">
                {engagements.data.slice(0, 6).map((engagement) => (
                  <li key={engagement.id}>
                    <Link
                      to={`/engagements/${engagement.id}`}
                      className="flex items-center justify-between gap-4 px-5 py-3 transition-colors hover:bg-slate-800/40"
                    >
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium text-slate-100">
                          {engagement.name}
                        </p>
                        <p className="mt-0.5 text-xs text-slate-500">
                          {engagement.clientCode || "No client code"} · {engagement.testCaseCount}{" "}
                          tests · {engagement.findingCount} findings
                        </p>
                      </div>
                      <div className="flex shrink-0 items-center gap-3">
                        <span className="text-xs text-slate-500">
                          {formatDate(engagement.updatedAt)}
                        </span>
                        <EngagementStatusBadge status={engagement.status} />
                      </div>
                    </Link>
                  </li>
                ))}
              </ul>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader title="Open findings by severity" description="Current exposure." />
          <CardBody className="space-y-4">
            {metrics.data &&
              (
                Object.keys(
                  metrics.data.findingsBySeverity,
                ) as (keyof typeof metrics.data.findingsBySeverity)[]
              )
                .slice()
                .reverse()
                .map((severity) => {
                  const count = metrics.data.findingsBySeverity[severity];
                  const total = Math.max(metrics.data.openFindings, 1);
                  return (
                    <div key={severity}>
                      <div className="mb-1 flex items-center justify-between">
                        <SeverityBadge severity={severity as Severity} />
                        <span className="text-xs font-medium text-slate-300">{count}</span>
                      </div>
                      <div className="h-1.5 overflow-hidden rounded-full bg-slate-800">
                        <div
                          className={`h-full rounded-full ${barColor(severity as Severity)}`}
                          style={{ width: `${Math.min(100, (count / total) * 100)}%` }}
                        />
                      </div>
                    </div>
                  );
                })}
            {metrics.data && metrics.data.openFindings === 0 && (
              <p className="py-6 text-center text-xs text-slate-500">
                No open findings. Nice work.
              </p>
            )}
          </CardBody>
        </Card>
      </div>

      <EngagementDialog open={createOpen} onOpenChange={setCreateOpen} />
    </div>
  );
}

function StatCard({
  icon,
  label,
  value,
  detail,
  tone = "default",
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  detail: string;
  tone?: "default" | "danger" | "warning";
}) {
  return (
    <Card>
      <CardBody>
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium uppercase tracking-wide text-slate-500">
            {label}
          </span>
          <span
            className={
              tone === "danger"
                ? "text-red-400"
                : tone === "warning"
                  ? "text-amber-400"
                  : "text-slate-500"
            }
          >
            {icon}
          </span>
        </div>
        <p className="mt-3 text-2xl font-semibold text-white">{value}</p>
        <p className="mt-1 text-xs text-slate-500">{detail}</p>
      </CardBody>
    </Card>
  );
}

function barColor(severity: Severity): string {
  switch (severity) {
    case "critical":
      return "bg-red-400";
    case "high":
      return "bg-orange-400";
    case "medium":
      return "bg-amber-400";
    case "low":
      return "bg-sky-400";
    default:
      return "bg-slate-400";
  }
}
