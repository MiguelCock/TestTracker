import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { Controller, useForm, useWatch } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { Pencil, Plus, ShieldAlert, Sparkles, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Badge, FindingStatusBadge, SeverityBadge } from "@/components/ui/badge";
import { Dialog } from "@/components/ui/dialog";
import { Field, Input, Textarea } from "@/components/ui/input";
import { Select, SelectItem } from "@/components/ui/select";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import { findingStatusLabel, retestStatusLabel, severityLabel } from "@/lib/labels";
import { severityFor } from "@/lib/severity";
import type { Finding, FindingInput, FindingStatus, RetestStatus, TestCase } from "@/types";

export function FindingsPanel({ engagementId }: { engagementId: string }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [createOpen, setCreateOpen] = useState(false);
  const [editing, setEditing] = useState<Finding | null>(null);
  const findings = useQuery({
    queryKey: ["findings", engagementId],
    queryFn: () => api.listFindings(engagementId),
  });
  const testCases = useQuery({
    queryKey: ["test-cases", engagementId],
    queryFn: () => api.listTestCases(engagementId),
  });

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["findings", engagementId] });
    void queryClient.invalidateQueries({ queryKey: ["engagement", engagementId] });
    void queryClient.invalidateQueries({ queryKey: ["engagements"] });
    void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
  };

  const remove = useMutation({
    mutationFn: (id: string) => api.deleteFinding(id),
    onSuccess: invalidate,
  });

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-white">Findings</h2>
          <p className="text-xs text-slate-500">
            Structured records with severity, remediation, retest status, and linked tests.
          </p>
        </div>
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus className="h-3.5 w-3.5" /> New finding
        </Button>
      </div>

      {findings.isLoading && <LoadingBlock label="Loading findings…" />}
      {findings.isError && <ErrorBlock message={findings.error.message} />}
      {remove.isError && <ErrorBlock message={remove.error.message} />}

      {findings.data && findings.data.length === 0 && (
        <EmptyState
          title="No findings yet"
          description="A passed test is not a finding. When a test exposes risk, create a structured finding with severity and remediation."
          action={
            <Button size="sm" onClick={() => setCreateOpen(true)}>
              <ShieldAlert className="h-3.5 w-3.5" /> Record finding
            </Button>
          }
        />
      )}

      {findings.data && findings.data.length > 0 && (
        <Card className="overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="border-b border-slate-800 bg-slate-950/40 text-[11px] uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Finding</th>
                  <th className="px-4 py-3 font-medium">Severity</th>
                  <th className="px-4 py-3 font-medium">Status</th>
                  <th className="px-4 py-3 font-medium">Retest</th>
                  <th className="px-4 py-3 font-medium">Owner</th>
                  <th className="px-4 py-3 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800">
                {findings.data.map((finding) => (
                  <tr
                    key={finding.id}
                    className="align-top transition-colors hover:bg-slate-800/20"
                  >
                    <td className="max-w-md px-4 py-3">
                      <p className="font-medium text-slate-100">
                        <span className="mr-2 text-xs text-slate-500">{finding.code}</span>
                        {finding.title}
                      </p>
                      <div className="mt-1 flex flex-wrap items-center gap-1.5">
                        {finding.owasp && <Badge tone="accent">{finding.owasp}</Badge>}
                        {finding.cwe && <Badge>{finding.cwe}</Badge>}
                        {finding.testCaseIds.length > 0 && (
                          <span className="text-[11px] text-slate-500">
                            {finding.testCaseIds.length} linked test
                            {finding.testCaseIds.length === 1 ? "" : "s"}
                          </span>
                        )}
                      </div>
                    </td>
                    <td className="px-4 py-3">
                      <SeverityBadge severity={finding.severity} />
                    </td>
                    <td className="px-4 py-3">
                      <FindingStatusBadge status={finding.status} />
                    </td>
                    <td className="px-4 py-3 text-xs text-slate-400">
                      {retestStatusLabel[finding.retestStatus]}
                    </td>
                    <td className="px-4 py-3 text-xs text-slate-400">{finding.owner || "—"}</td>
                    <td className="px-4 py-3">
                      <div className="flex justify-end gap-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label="Ask assistant about finding"
                          onClick={() =>
                            navigate(
                              `/engagements/${engagementId}?tab=assistant&focus=finding:${finding.id}`,
                            )
                          }
                        >
                          <Sparkles className="h-3.5 w-3.5 text-emerald-400" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label="Edit finding"
                          onClick={() => setEditing(finding)}
                        >
                          <Pencil className="h-3.5 w-3.5" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label="Delete finding"
                          onClick={() => {
                            if (window.confirm(`Delete ${finding.code} "${finding.title}"?`)) {
                              remove.mutate(finding.id);
                            }
                          }}
                        >
                          <Trash2 className="h-3.5 w-3.5 text-slate-500" />
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      )}

      {createOpen && (
        <FindingDialog
          open={createOpen}
          onOpenChange={setCreateOpen}
          engagementId={engagementId}
          finding={null}
          testCases={testCases.data ?? []}
          onSaved={invalidate}
        />
      )}
      {editing && (
        <FindingDialog
          key={editing.id}
          open
          onOpenChange={(open) => !open && setEditing(null)}
          engagementId={engagementId}
          finding={editing}
          testCases={testCases.data ?? []}
          onSaved={invalidate}
        />
      )}
    </div>
  );
}

const findingSchema = z.object({
  code: z.string().max(40),
  title: z.string().trim().min(1, "Enter a finding title.").max(300),
  description: z.string().max(50_000),
  impact: z.string().max(50_000),
  remediation: z.string().max(50_000),
  owasp: z.string().max(200),
  cwe: z.string().max(200),
  status: z.enum(["open", "in_review", "remediated", "retest_pending", "closed", "accepted"]),
  retestStatus: z.enum(["not_required", "pending", "passed", "failed"]),
  owner: z.string().max(120),
  probability: z.number().int().min(0).max(9),
  impactScore: z.number().int().min(0).max(9),
  testCaseIds: z.array(z.string()),
});

type FindingValues = z.infer<typeof findingSchema>;

function FindingDialog({
  open,
  onOpenChange,
  engagementId,
  finding,
  testCases,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  engagementId: string;
  finding: Finding | null;
  testCases: TestCase[];
  onSaved: () => void;
}) {
  const [serverError, setServerError] = useState<string | null>(null);
  const form = useForm<FindingValues>({
    resolver: zodResolver(findingSchema),
    defaultValues: valuesFor(finding),
  });
  useEffect(() => {
    if (open) {
      form.reset(valuesFor(finding));
    }
  }, [open, finding, form]);
  const probability = Number(useWatch({ control: form.control, name: "probability" }));
  const impact = Number(useWatch({ control: form.control, name: "impactScore" }));
  const severity = severityFor(
    Math.min(9, Math.max(0, Number.isFinite(probability) ? probability : 0)),
    Math.min(9, Math.max(0, Number.isFinite(impact) ? impact : 0)),
  );

  const mutation = useMutation({
    mutationFn: (values: FindingValues) => {
      const input: FindingInput = {
        engagementId,
        code: values.code.trim(),
        title: values.title.trim(),
        description: values.description,
        impact: values.impact,
        remediation: values.remediation,
        owasp: values.owasp.trim(),
        cwe: values.cwe.trim(),
        status: values.status,
        retestStatus: values.retestStatus,
        owner: values.owner.trim(),
        probability: values.probability,
        impactScore: values.impactScore,
        testCaseIds: values.testCaseIds,
      };
      return finding ? api.updateFinding(finding.id, input) : api.createFinding(input);
    },
    onSuccess: () => {
      onSaved();
      onOpenChange(false);
    },
    onError: (error) => setServerError(error.message),
  });

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setServerError(null);
        onOpenChange(next);
      }}
      title={finding ? `Edit ${finding.code}` : "New finding"}
      description="Findings are first-class records; severity is calculated from probability and impact."
      wide
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="submit" form="finding-form" disabled={mutation.isPending}>
            {mutation.isPending && <Spinner />}
            Save finding
          </Button>
        </>
      }
    >
      <form
        id="finding-form"
        className="space-y-4"
        onSubmit={form.handleSubmit((values) => {
          setServerError(null);
          mutation.mutate(values);
        })}
      >
        <div className="grid gap-4 sm:grid-cols-[140px_1fr_150px_150px]">
          <Field label="Code" hint="Auto if blank">
            <Input placeholder="F-001" {...form.register("code")} />
          </Field>
          <Field label="Title" error={form.formState.errors.title?.message}>
            <Input autoFocus {...form.register("title")} />
          </Field>
          <Field label="Probability (0-9)">
            <Input
              type="number"
              min={0}
              max={9}
              {...form.register("probability", { valueAsNumber: true })}
            />
          </Field>
          <Field label="Impact (0-9)">
            <Input
              type="number"
              min={0}
              max={9}
              {...form.register("impactScore", { valueAsNumber: true })}
            />
          </Field>
        </div>

        <div className="flex items-center gap-2">
          <span className="text-xs text-slate-500">Calculated severity</span>
          <SeverityBadge severity={severity} />
          <span className="text-[11px] text-slate-600">
            {severityLabel[severity]} · {probability} × {impact}
          </span>
        </div>

        <Field label="Description">
          <Textarea
            rows={3}
            placeholder="What was observed and why it matters."
            {...form.register("description")}
          />
        </Field>
        <Field label="Impact">
          <Textarea rows={2} {...form.register("impact")} />
        </Field>
        <Field label="Remediation">
          <Textarea rows={2} {...form.register("remediation")} />
        </Field>

        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <Field label="OWASP">
            <Input placeholder="A1: Broken Access Control" {...form.register("owasp")} />
          </Field>
          <Field label="CWE">
            <Input placeholder="CWE-639" {...form.register("cwe")} />
          </Field>
          <Field label="Status">
            <Controller
              control={form.control}
              name="status"
              render={({ field }) => (
                <Select value={field.value} onValueChange={field.onChange}>
                  {(Object.keys(findingStatusLabel) as FindingStatus[]).map((status) => (
                    <SelectItem key={status} value={status}>
                      {findingStatusLabel[status]}
                    </SelectItem>
                  ))}
                </Select>
              )}
            />
          </Field>
          <Field label="Retest">
            <Controller
              control={form.control}
              name="retestStatus"
              render={({ field }) => (
                <Select value={field.value} onValueChange={field.onChange}>
                  {(Object.keys(retestStatusLabel) as RetestStatus[]).map((status) => (
                    <SelectItem key={status} value={status}>
                      {retestStatusLabel[status]}
                    </SelectItem>
                  ))}
                </Select>
              )}
            />
          </Field>
        </div>

        <Field label="Owner">
          <Input {...form.register("owner")} />
        </Field>

        <div>
          <p className="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
            Linked test cases
          </p>
          {testCases.length === 0 ? (
            <p className="text-xs text-slate-500">
              No test cases yet. The finding can still be saved and linked later.
            </p>
          ) : (
            <div className="max-h-44 space-y-1 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950/40 p-2">
              {testCases.map((testCase) => (
                <label
                  key={testCase.id}
                  className="flex cursor-pointer items-center gap-2 rounded px-2 py-1 text-xs text-slate-300 hover:bg-slate-800/50"
                >
                  <input
                    type="checkbox"
                    className="h-3.5 w-3.5 rounded border-slate-600 bg-slate-900"
                    value={testCase.id}
                    {...form.register("testCaseIds")}
                  />
                  <span className="truncate">{testCase.title}</span>
                </label>
              ))}
            </div>
          )}
        </div>

        {serverError && (
          <p className="rounded-lg border border-red-500/30 bg-red-500/5 px-3 py-2 text-xs text-red-300">
            {serverError}
          </p>
        )}
        <button type="submit" className="hidden" />
      </form>
    </Dialog>
  );
}

function valuesFor(finding: Finding | null): FindingValues {
  return {
    code: finding?.code ?? "",
    title: finding?.title ?? "",
    description: finding?.description ?? "",
    impact: finding?.impact ?? "",
    remediation: finding?.remediation ?? "",
    owasp: finding?.owasp ?? "",
    cwe: finding?.cwe ?? "",
    status: finding?.status ?? "open",
    retestStatus: finding?.retestStatus ?? "not_required",
    owner: finding?.owner ?? "",
    probability: finding?.probability ?? 5,
    impactScore: finding?.impactScore ?? 5,
    testCaseIds: finding?.testCaseIds ?? [],
  };
}
