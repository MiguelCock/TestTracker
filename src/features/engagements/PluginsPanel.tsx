import { useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Play, Plus, Puzzle, ShieldCheck } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import type { PluginFindingSuggestion, PluginRunResult } from "@/types";

const severityTone: Record<string, "neutral" | "info" | "warning" | "danger"> = {
  informational: "neutral",
  low: "info",
  medium: "warning",
  high: "danger",
  critical: "danger",
};

export function PluginsPanel({ engagementId }: { engagementId: string }) {
  const queryClient = useQueryClient();
  const plugins = useQuery({ queryKey: ["plugins"], queryFn: api.listPlugins });
  const [pluginId, setPluginId] = useState("");
  const [result, setResult] = useState<PluginRunResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [added, setAdded] = useState(0);

  const enabled = (plugins.data ?? []).filter((plugin) => plugin.enabled);
  const selectedId = pluginId || enabled[0]?.id || "";
  const selected = enabled.find((plugin) => plugin.id === selectedId);

  const run = useMutation({
    mutationFn: () => api.runPlugin(selectedId, engagementId),
    onSuccess: (output) => {
      setError(null);
      setAdded(0);
      setResult(output);
      void queryClient.invalidateQueries({ queryKey: ["audit", engagementId] });
    },
    onError: (mutationError) => {
      setResult(null);
      setError(mutationError.message);
    },
  });

  const addSuggested = useMutation({
    mutationFn: async (output: PluginRunResult) => {
      for (const [index, testCase] of output.testCases.entries()) {
        await api.createTestCase({
          engagementId,
          templateId: null,
          title: testCase.title,
          category: testCase.category,
          approach: testCase.approach,
          status: "not_started",
          result: "",
          notes: testCase.expectedResult,
          assignee: "",
          sortOrder: index,
        });
      }
      return output.testCases.length;
    },
    onSuccess: (count) => {
      setAdded(count);
      void queryClient.invalidateQueries({ queryKey: ["test-cases", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["engagement", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["engagements"] });
      void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-white">Plugins</h2>
          <p className="text-xs text-slate-500">
            Signed WebAssembly modules run in a sandbox with declared capabilities.
          </p>
        </div>
        {enabled.length > 0 && (
          <div className="flex items-center gap-2">
            <select
              value={selectedId}
              onChange={(event) => setPluginId(event.target.value)}
              className="h-9 rounded-lg border border-slate-700 bg-slate-950/60 px-3 text-xs text-slate-200"
            >
              {enabled.map((plugin) => (
                <option key={plugin.id} value={plugin.id}>
                  {plugin.name} · v{plugin.version}
                </option>
              ))}
            </select>
            <Button size="sm" onClick={() => run.mutate()} disabled={run.isPending || !selectedId}>
              {run.isPending ? <Spinner /> : <Play className="h-3.5 w-3.5" />}
              Run plugin
            </Button>
          </div>
        )}
      </div>

      {error && <ErrorBlock message={error} />}
      {plugins.isLoading && <LoadingBlock label="Loading plugins…" />}
      {plugins.isError && <ErrorBlock message={plugins.error.message} />}
      {plugins.data && enabled.length === 0 && (
        <EmptyState
          title="No enabled plugins"
          description="Install and enable a signed plugin in Settings, then return here to run it against this engagement."
          action={
            <Link
              to="/settings"
              className="inline-flex h-9 items-center rounded-lg bg-emerald-500 px-4 text-xs font-medium text-slate-950 hover:bg-emerald-400"
            >
              Open settings
            </Link>
          }
        />
      )}

      {selected && (
        <div className="flex flex-wrap items-center gap-2 text-[11px] text-slate-500">
          <Puzzle className="h-3.5 w-3.5" />
          <span>{selected.id}</span>
          <span>·</span>
          <span>publisher {selected.publisherId}</span>
          {selected.capabilities.map((capability) => (
            <Badge key={capability} tone="accent">
              {capability}
            </Badge>
          ))}
        </div>
      )}

      {result && (
        <PluginOutput
          result={result}
          added={added}
          adding={addSuggested.isPending}
          onAdd={() => addSuggested.mutate(result)}
        />
      )}

      <div className="flex items-start gap-2 rounded-lg border border-slate-800 bg-slate-900/40 p-3 text-[11px] text-slate-500">
        <ShieldCheck className="mt-0.5 h-3.5 w-3.5 shrink-0 text-emerald-500/70" />
        Plugins receive no evidence content and cannot write to the database. Suggested test cases
        are added only when you choose to add them. Execution is fuel- and memory-limited.
      </div>
    </div>
  );
}

function PluginOutput({
  result,
  added,
  adding,
  onAdd,
}: {
  result: PluginRunResult;
  added: number;
  adding: boolean;
  onAdd: () => void;
}) {
  return (
    <div className="space-y-4">
      <Card>
        <CardHeader
          title={`${result.pluginName} v${result.pluginVersion}`}
          description={`${result.fuelUsed.toLocaleString()} fuel used · ${result.logs.length} log(s)`}
          action={
            result.testCases.length > 0 ? (
              <Button size="sm" variant="secondary" onClick={onAdd} disabled={adding || added > 0}>
                {adding ? <Spinner /> : <Plus className="h-3.5 w-3.5" />}
                {added > 0 ? `${added} added` : "Add suggested tests"}
              </Button>
            ) : undefined
          }
        />
        <CardBody className="space-y-4">
          {result.summary && (
            <p className="whitespace-pre-wrap text-sm leading-relaxed text-slate-200">
              {result.summary}
            </p>
          )}

          {result.testCases.length > 0 && (
            <div>
              <p className="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
                Suggested test cases
              </p>
              <div className="overflow-hidden rounded-lg border border-slate-800">
                <table className="w-full text-left text-xs">
                  <thead className="border-b border-slate-800 bg-slate-950/40 text-[10px] uppercase tracking-wide text-slate-500">
                    <tr>
                      <th className="px-3 py-2 font-medium">Test</th>
                      <th className="px-3 py-2 font-medium">Category</th>
                      <th className="px-3 py-2 font-medium">Approach</th>
                      <th className="px-3 py-2 font-medium">Expected result</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-800">
                    {result.testCases.map((testCase, index) => (
                      <tr key={`${testCase.title}-${index}`}>
                        <td className="px-3 py-2 font-medium text-slate-200">{testCase.title}</td>
                        <td className="px-3 py-2 text-slate-400">{testCase.category || "—"}</td>
                        <td className="px-3 py-2 text-slate-400">{testCase.approach || "—"}</td>
                        <td className="px-3 py-2 text-slate-400">
                          {testCase.expectedResult || "—"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {result.findings.length > 0 && (
            <div>
              <p className="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
                Suggested findings
              </p>
              <div className="space-y-2">
                {result.findings.map((finding, index) => (
                  <FindingSuggestion key={`${finding.title}-${index}`} finding={finding} />
                ))}
              </div>
            </div>
          )}

          {result.notes.length > 0 && (
            <div>
              <p className="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
                Notes
              </p>
              <ul className="space-y-1">
                {result.notes.map((note, index) => (
                  <li key={index} className="flex gap-2 text-xs text-slate-300">
                    <span className="mt-1 h-1 w-1 shrink-0 rounded-full bg-emerald-400" />
                    {note}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {result.logs.length > 0 && (
            <details>
              <summary className="cursor-pointer text-[11px] text-slate-500">
                Plugin logs ({result.logs.length})
              </summary>
              <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-950 p-3 text-[10px] text-slate-400">
                {result.logs.join("\n")}
              </pre>
            </details>
          )}
        </CardBody>
      </Card>
    </div>
  );
}

function FindingSuggestion({ finding }: { finding: PluginFindingSuggestion }) {
  const severity = finding.severity.toLowerCase();
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950/40 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <p className="text-xs font-medium text-slate-200">{finding.title}</p>
        {finding.severity && (
          <Badge tone={severityTone[severity] ?? "neutral"}>{finding.severity}</Badge>
        )}
      </div>
      {finding.description && (
        <p className="mt-1 text-[11px] leading-relaxed text-slate-400">{finding.description}</p>
      )}
      {finding.impact && (
        <p className="mt-1 text-[11px] text-slate-500">Impact: {finding.impact}</p>
      )}
      {finding.remediation && (
        <p className="mt-1 text-[11px] text-slate-500">Remediation: {finding.remediation}</p>
      )}
      <p className="mt-2 text-[10px] text-slate-600">
        Suggested only. Create a finding manually after reviewing the evidence.
      </p>
    </div>
  );
}
