import { useState } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  CalendarDays,
  Download,
  Pencil,
  Sparkles,
  ShieldAlert,
  Trash2,
  Users,
} from "lucide-react";
import { Badge, EngagementStatusBadge, SeverityBadge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { EmptyState, ErrorBlock, LoadingBlock } from "@/components/ui/states";
import { KeyValue, PageHeader } from "@/components/layout/PageHeader";
import { api } from "@/lib/api";
import { formatDate, formatDateTime } from "@/lib/format";
import { EngagementDialog } from "@/features/engagements/EngagementDialog";
import { TestTrackerPanel } from "@/features/engagements/TestTrackerPanel";
import { FindingsPanel } from "@/features/engagements/FindingsPanel";
import { EvidencePanel } from "@/features/engagements/EvidencePanel";
import { ReviewPanel } from "@/features/engagements/ReviewPanel";
import { DocumentsPanel } from "@/features/engagements/DocumentsPanel";
import { AssistantPanel } from "@/features/ai/AssistantPanel";
import { PluginsPanel } from "@/features/engagements/PluginsPanel";
import { ExportBundleDialog } from "@/features/bundles/ExportBundleDialog";
import { AuditPanel } from "@/features/engagements/AuditPanel";
import { AgentSkillDialog } from "@/features/ai/AgentSkillDialog";
import type { AgentSkill, AssistantFocus, EngagementDetail } from "@/types";

export function EngagementDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const activeTab = searchParams.get("tab") ?? "assistant";
  const assistantFocus = parseAssistantFocus(searchParams.get("focus"));
  const [editOpen, setEditOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [agentSkill, setAgentSkill] = useState<AgentSkill | null>(null);

  const detail = useQuery({
    queryKey: ["engagement", id],
    queryFn: () => api.getEngagement(id!),
    enabled: Boolean(id),
  });

  const remove = useMutation({
    mutationFn: () => api.deleteEngagement(id!),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["engagements"] });
      void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
      navigate("/engagements");
    },
  });

  if (detail.isLoading) return <LoadingBlock label="Loading engagement…" />;
  if (detail.isError) return <ErrorBlock message={detail.error.message} />;
  if (!detail.data || !id) return null;

  const { engagement } = detail.data;

  return (
    <div>
      <Link
        to="/engagements"
        className="mb-4 inline-flex items-center gap-1 text-xs text-slate-500 hover:text-slate-300"
      >
        <ArrowLeft className="h-3.5 w-3.5" /> All engagements
      </Link>

      <PageHeader
        title={engagement.name}
        description={`${engagement.clientCode || "No client code"} · created ${formatDateTime(
          engagement.createdAt,
        )}`}
        actions={
          <>
            <Button variant="secondary" onClick={() => setExportOpen(true)}>
              <Download className="h-4 w-4" /> Export bundle
            </Button>
            <Button variant="secondary" onClick={() => setEditOpen(true)}>
              <Pencil className="h-4 w-4" /> Edit
            </Button>
            <Button
              variant="danger"
              onClick={() => {
                if (
                  window.confirm(
                    `Delete "${engagement.name}"? This removes its tests, findings, and audit history.`,
                  )
                ) {
                  remove.mutate();
                }
              }}
              disabled={remove.isPending}
            >
              <Trash2 className="h-4 w-4" /> Delete
            </Button>
          </>
        }
      />

      {remove.isError && <ErrorBlock message={remove.error.message} />}

      <div className="mb-6 flex flex-wrap items-center gap-3">
        <EngagementStatusBadge status={engagement.status} />
        <span className="inline-flex items-center gap-1.5 text-xs text-slate-400">
          <CalendarDays className="h-3.5 w-3.5" />
          {formatDate(engagement.startDate)} – {formatDate(engagement.endDate)}
        </span>
        <span className="text-xs text-slate-500">v{engagement.version}</span>
        <span className="inline-flex items-center gap-1.5 text-xs text-slate-400">
          <Users className="h-3.5 w-3.5" /> {detail.data.testCaseCount} tests
        </span>
        <span className="inline-flex items-center gap-1.5 text-xs text-slate-400">
          <ShieldAlert className="h-3.5 w-3.5" /> {detail.data.openFindingCount} open findings
        </span>
      </div>

      <Tabs
        value={activeTab}
        onValueChange={(value) => {
          const params = new URLSearchParams(searchParams);
          params.set("tab", value);
          if (value !== "assistant") params.delete("focus");
          setSearchParams(params);
        }}
      >
        <TabsList>
          <TabsTrigger value="assistant">Assistant</TabsTrigger>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="tests">Test tracker</TabsTrigger>
          <TabsTrigger value="findings">Findings</TabsTrigger>
          <TabsTrigger value="evidence">Evidence</TabsTrigger>
          <TabsTrigger value="review">Review</TabsTrigger>
          <TabsTrigger value="documents">Documents</TabsTrigger>
          <TabsTrigger value="plugins">Plugins</TabsTrigger>
          <TabsTrigger value="audit">Audit</TabsTrigger>
        </TabsList>

        <div className="mt-4">
          <TabsContent value="assistant">
            <AssistantPanel
              engagementId={id}
              focus={assistantFocus}
              onFocusChange={(next) => {
                const params = new URLSearchParams(searchParams);
                if (next.kind === "engagement") {
                  params.delete("focus");
                } else {
                  params.set("focus", `${next.kind}:${next.id}`);
                }
                params.set("tab", "assistant");
                setSearchParams(params);
              }}
            />
          </TabsContent>
          <TabsContent value="overview">
            <OverviewPanel detail={detail.data} onDraft={setAgentSkill} />
          </TabsContent>
          <TabsContent value="tests">
            <TestTrackerPanel engagementId={id} />
          </TabsContent>
          <TabsContent value="findings">
            <FindingsPanel engagementId={id} />
          </TabsContent>
          <TabsContent value="evidence">
            <EvidencePanel engagementId={id} />
          </TabsContent>
          <TabsContent value="review">
            <ReviewPanel engagementId={id} />
          </TabsContent>
          <TabsContent value="documents">
            <DocumentsPanel engagementId={id} />
          </TabsContent>
          <TabsContent value="plugins">
            <PluginsPanel engagementId={id} />
          </TabsContent>
          <TabsContent value="audit">
            <AuditPanel engagementId={id} />
          </TabsContent>
        </div>
      </Tabs>

      <EngagementDialog open={editOpen} onOpenChange={setEditOpen} detail={detail.data} />
      {exportOpen && (
        <ExportBundleDialog
          open={exportOpen}
          onOpenChange={setExportOpen}
          engagementId={id}
          engagementName={engagement.name}
        />
      )}
      {agentSkill && (
        <AgentSkillDialog
          open
          onOpenChange={(next) => !next && setAgentSkill(null)}
          engagementId={id}
          skills={[agentSkill]}
          onApplied={() => {
            void queryClient.invalidateQueries({ queryKey: ["engagement", id] });
            void queryClient.invalidateQueries({ queryKey: ["engagements"] });
            void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
            void queryClient.invalidateQueries({ queryKey: ["audit", id] });
          }}
        />
      )}
    </div>
  );
}

function OverviewPanel({
  detail,
  onDraft,
}: {
  detail: EngagementDetail;
  onDraft: (skill: AgentSkill) => void;
}) {
  const { engagement, scopeAssets, risks, assumptions, contacts } = detail;
  return (
    <div className="grid gap-4 xl:grid-cols-[1.4fr_1fr]">
      <div className="space-y-4">
        <Card>
          <CardHeader
            title="Application summary"
            action={
              <Button variant="ghost" size="sm" onClick={() => onDraft("application_summary")}>
                <Sparkles className="h-3.5 w-3.5" /> Draft
              </Button>
            }
          />
          <CardBody>
            {engagement.applicationSummary ? (
              <p className="whitespace-pre-wrap text-sm leading-relaxed text-slate-300">
                {engagement.applicationSummary}
              </p>
            ) : (
              <p className="text-xs text-slate-500">No summary recorded.</p>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader
            title="Attack surface"
            action={
              <Button variant="ghost" size="sm" onClick={() => onDraft("attack_surface")}>
                <Sparkles className="h-3.5 w-3.5" /> Draft
              </Button>
            }
          />
          <CardBody>
            {engagement.attackSurface ? (
              <p className="whitespace-pre-wrap text-sm leading-relaxed text-slate-300">
                {engagement.attackSurface}
              </p>
            ) : (
              <p className="text-xs text-slate-500">No attack surface recorded.</p>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader title="Scope assets" description="What is and is not in scope." />
          <CardBody className="p-0">
            {scopeAssets.length === 0 ? (
              <div className="p-5">
                <EmptyState
                  title="No scope assets"
                  description="Edit the engagement to add assets."
                />
              </div>
            ) : (
              <table className="w-full text-left text-sm">
                <thead className="border-b border-slate-800 bg-slate-950/40 text-[11px] uppercase tracking-wide text-slate-500">
                  <tr>
                    <th className="px-5 py-2.5 font-medium">Kind</th>
                    <th className="px-5 py-2.5 font-medium">Value</th>
                    <th className="px-5 py-2.5 font-medium">Scope</th>
                    <th className="px-5 py-2.5 font-medium">Notes</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-800">
                  {scopeAssets.map((asset) => (
                    <tr key={asset.id}>
                      <td className="px-5 py-2.5">
                        <Badge>{asset.kind.replace("_", " ")}</Badge>
                      </td>
                      <td className="px-5 py-2.5 font-mono text-xs text-slate-300">
                        {asset.value}
                      </td>
                      <td className="px-5 py-2.5 text-xs">
                        {asset.inScope ? (
                          <span className="text-emerald-300">In scope</span>
                        ) : (
                          <span className="text-slate-500">Out of scope</span>
                        )}
                      </td>
                      <td className="px-5 py-2.5 text-xs text-slate-500">{asset.notes || "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader title="Risks" description="Scoping risks and their calculated severity." />
          <CardBody className="p-0">
            {risks.length === 0 ? (
              <div className="p-5">
                <EmptyState title="No risks recorded" />
              </div>
            ) : (
              <table className="w-full text-left text-sm">
                <thead className="border-b border-slate-800 bg-slate-950/40 text-[11px] uppercase tracking-wide text-slate-500">
                  <tr>
                    <th className="px-5 py-2.5 font-medium">Risk</th>
                    <th className="px-5 py-2.5 font-medium">L × I</th>
                    <th className="px-5 py-2.5 font-medium">Severity</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-800">
                  {risks.map((risk) => (
                    <tr key={risk.id}>
                      <td className="px-5 py-2.5">
                        <p className="text-xs font-medium text-slate-200">{risk.title}</p>
                        {risk.description && (
                          <p className="mt-0.5 text-[11px] text-slate-500">{risk.description}</p>
                        )}
                      </td>
                      <td className="px-5 py-2.5 text-xs text-slate-400">
                        {risk.likelihood} × {risk.impact}
                      </td>
                      <td className="px-5 py-2.5">
                        <SeverityBadge severity={risk.severity} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </CardBody>
        </Card>
      </div>

      <div className="space-y-4">
        <Card>
          <CardHeader
            title="Assumptions"
            action={
              <Button variant="ghost" size="sm" onClick={() => onDraft("assumptions")}>
                <Sparkles className="h-3.5 w-3.5" /> Draft
              </Button>
            }
          />
          <CardBody>
            {assumptions.length === 0 ? (
              <p className="text-xs text-slate-500">No assumptions recorded.</p>
            ) : (
              <ul className="space-y-2">
                {assumptions.map((assumption) => (
                  <li key={assumption.id} className="flex gap-2 text-xs text-slate-300">
                    <span className="mt-1 h-1 w-1 shrink-0 rounded-full bg-emerald-400" />
                    {assumption.text}
                  </li>
                ))}
              </ul>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader title="Contacts" />
          <CardBody>
            {contacts.length === 0 ? (
              <p className="text-xs text-slate-500">No contacts recorded.</p>
            ) : (
              <div className="space-y-3">
                {contacts.map((contact) => (
                  <div
                    key={contact.id}
                    className="border-b border-slate-800 pb-3 last:border-0 last:pb-0"
                  >
                    <p className="text-xs font-medium text-slate-200">{contact.name}</p>
                    <p className="text-[11px] text-slate-500">{contact.role || "Contact"}</p>
                    {(contact.email || contact.phone) && (
                      <p className="mt-1 text-[11px] text-slate-400">
                        {[contact.email, contact.phone].filter(Boolean).join(" · ")}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader title="Engagement facts" />
          <CardBody className="grid grid-cols-2 gap-4">
            <KeyValue label="Created">{formatDateTime(engagement.createdAt)}</KeyValue>
            <KeyValue label="Updated">{formatDateTime(engagement.updatedAt)}</KeyValue>
            <KeyValue label="Tests">{detail.testCaseCount}</KeyValue>
            <KeyValue label="Findings">{detail.findingCount}</KeyValue>
            <KeyValue label="Open">{detail.openFindingCount}</KeyValue>
            <KeyValue label="Version">v{engagement.version}</KeyValue>
          </CardBody>
        </Card>
      </div>
    </div>
  );
}

function parseAssistantFocus(value: string | null): AssistantFocus {
  if (!value) return { kind: "engagement", id: null };
  const separator = value.indexOf(":");
  if (separator < 0) return { kind: "engagement", id: null };
  const kind = value.slice(0, separator);
  const id = value.slice(separator + 1);
  if (!id || !["test_case", "finding", "evidence"].includes(kind)) {
    return { kind: "engagement", id: null };
  }
  return { kind: kind as AssistantFocus["kind"], id };
}
