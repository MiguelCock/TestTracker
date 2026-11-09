import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bot, CheckCircle2, FileUp, ShieldCheck, Sparkles } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Field, Textarea } from "@/components/ui/input";
import { Select, SelectItem } from "@/components/ui/select";
import { ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import { readFileAsBase64 } from "@/lib/base64";
import { formatBytes } from "@/lib/format";
import type {
  ImportApplyResult,
  ImportDraft,
  ImportDraftResult,
  ImportExtraction,
  ImportPreview,
} from "@/types";

const MAX_IMPORT_BYTES = 25 * 1024 * 1024;
const ACCEPTED_FILES = ".xlsx,.xlsm,.xls,.docx,.pdf,.md,.markdown,.json,.txt,.text,.log";

export function ImportAssessmentDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [file, setFile] = useState<File | null>(null);
  const [extraction, setExtraction] = useState<ImportExtraction | null>(null);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [draft, setDraft] = useState<ImportDraftResult | null>(null);
  const [result, setResult] = useState<ImportApplyResult | null>(null);
  const [providerId, setProviderId] = useState("");
  const [allowCloud, setAllowCloud] = useState(false);
  const [instructions, setInstructions] = useState("");
  const [error, setError] = useState<string | null>(null);

  const providers = useQuery({ queryKey: ["ai-providers"], queryFn: api.listAiProviders });
  const selectedProviderId = providerId || providers.data?.[0]?.id || "";
  const selectedProvider = providers.data?.find((provider) => provider.id === selectedProviderId);
  const requiresCloudConsent = Boolean(selectedProvider && !selectedProvider.isLocal);
  const extractMutation = useMutation({
    mutationFn: async () => {
      if (!file) throw new Error("Choose an assessment file.");
      if (file.size > MAX_IMPORT_BYTES) {
        throw new Error("Import files must be 25 MB or smaller.");
      }
      const data = await readFileAsBase64(file);
      return api.extractImportDocument(file.name, data);
    },
    onSuccess: (data) => {
      setError(null);
      setExtraction(data);
      setPreview(null);
      setDraft(data.draft && data.draftId ? { draftId: data.draftId, draft: data.draft } : null);
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const previewMutation = useMutation({
    mutationFn: () => {
      if (!extraction) throw new Error("Extract a document first.");
      return api.previewImportAi(
        extraction.extractionId,
        selectedProviderId,
        allowCloud,
        instructions,
      );
    },
    onSuccess: (data) => {
      setError(null);
      setPreview(data);
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const canPreviewAi =
    Boolean(extraction) &&
    Boolean(selectedProvider) &&
    (!requiresCloudConsent || allowCloud) &&
    !previewMutation.isPending;

  const runMutation = useMutation({
    mutationFn: () => api.runImportAi(preview!.consentId),
    onSuccess: (data) => {
      setError(null);
      setPreview(null);
      setDraft(data);
    },
    onError: (mutationError) => {
      setPreview(null);
      setError(mutationError.message);
    },
  });

  const applyMutation = useMutation({
    mutationFn: () => api.applyImportDraft(draft!.draftId),
    onSuccess: (data) => {
      setError(null);
      setResult(data);
      void queryClient.invalidateQueries({ queryKey: ["engagements"] });
      void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const reset = () => {
    setFile(null);
    setExtraction(null);
    setPreview(null);
    setDraft(null);
    setResult(null);
    setAllowCloud(false);
    setInstructions("");
    setError(null);
  };

  const close = () => {
    onOpenChange(false);
  };

  const title = result
    ? "Import complete"
    : preview
      ? "Review AI import"
      : draft
        ? "Review import draft"
        : "Import previous assessment";

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          setError(null);
          if (result) reset();
        }
        onOpenChange(next);
      }}
      title={title}
      description="Create a new engagement from a previous assessment. Nothing is written until you review and apply the draft."
      wide
      footer={
        result ? (
          <>
            <Button variant="secondary" onClick={close}>
              Close
            </Button>
            <Button
              onClick={() => {
                close();
                navigate(`/engagements/${result.engagementId}`);
              }}
            >
              Open engagement
            </Button>
          </>
        ) : preview ? (
          <>
            <Button variant="secondary" onClick={() => setPreview(null)}>
              Back
            </Button>
            <Button onClick={() => runMutation.mutate()} disabled={runMutation.isPending}>
              {runMutation.isPending ? <Spinner /> : <Bot className="h-4 w-4" />}
              Run AI import
            </Button>
          </>
        ) : draft ? (
          <>
            <Button variant="secondary" onClick={() => setDraft(null)}>
              Back
            </Button>
            <Button onClick={() => applyMutation.mutate()} disabled={applyMutation.isPending}>
              {applyMutation.isPending ? <Spinner /> : <CheckCircle2 className="h-4 w-4" />}
              Create engagement
            </Button>
          </>
        ) : extraction ? (
          <>
            <Button variant="secondary" onClick={close}>
              Cancel
            </Button>
            <Button onClick={() => previewMutation.mutate()} disabled={!canPreviewAi}>
              {previewMutation.isPending ? <Spinner /> : <ShieldCheck className="h-4 w-4" />}
              Preview AI import
            </Button>
          </>
        ) : (
          <>
            <Button variant="secondary" onClick={close}>
              Cancel
            </Button>
            <Button
              onClick={() => extractMutation.mutate()}
              disabled={!file || extractMutation.isPending}
            >
              {extractMutation.isPending ? <Spinner /> : <FileUp className="h-4 w-4" />}
              Extract document
            </Button>
          </>
        )
      }
    >
      {result ? (
        <div className="space-y-4">
          <div className="flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-xs text-emerald-300">
            <CheckCircle2 className="h-4 w-4" /> Assessment imported successfully.
          </div>
          <dl className="grid grid-cols-2 gap-3 text-xs">
            <div>
              <dt className="text-slate-500">Engagement</dt>
              <dd className="mt-0.5 text-slate-200">{result.engagementName}</dd>
            </div>
            <div>
              <dt className="text-slate-500">Contents</dt>
              <dd className="mt-0.5 text-slate-200">
                {result.testCaseCount} tests, {result.findingCount} findings
              </dd>
            </div>
          </dl>
          <p className="text-[11px] text-slate-500">
            The engagement was created in draft status. Review imported records, evidence, and
            findings before sharing or reporting.
          </p>
        </div>
      ) : preview ? (
        <div className="space-y-4">
          <div className="flex flex-wrap items-center gap-2">
            <Badge tone="accent">{preview.providerName}</Badge>
            <Badge>{preview.model}</Badge>
            <Badge tone={preview.isLocal ? "success" : "warning"}>
              {preview.isLocal ? "Local" : "Cloud"}
            </Badge>
            {preview.redactionCount > 0 && (
              <Badge tone="danger">{preview.redactionCount} redaction(s)</Badge>
            )}
          </div>
          {preview.redactions.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {preview.redactions.map((label) => (
                <Badge key={label} tone="danger">
                  {label}
                </Badge>
              ))}
            </div>
          )}
          <pre className="max-h-[50vh] overflow-auto whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs leading-relaxed text-slate-200">
            {preview.redactedPrompt}
          </pre>
          <p className="text-[11px] text-slate-500">
            This preview is the consent. Running the import makes one provider call. The returned
            draft still requires explicit approval.
          </p>
          {error && <ErrorBlock message={error} />}
        </div>
      ) : draft ? (
        <DraftReview draft={draft.draft} error={error} />
      ) : extraction ? (
        <div className="space-y-4">
          <div className="flex flex-wrap items-center gap-2">
            <Badge tone="info">{extraction.format.toUpperCase()}</Badge>
            <Badge>{extraction.textChars.toLocaleString()} chars</Badge>
            {extraction.truncated && <Badge tone="warning">Text truncated</Badge>}
            <span className="text-xs text-slate-500">{extraction.filename}</span>
          </div>
          <pre className="max-h-56 overflow-auto whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs leading-relaxed text-slate-300">
            {extraction.textPreview}
          </pre>
          {extraction.draft && extraction.draftId && (
            <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-3">
              <div className="text-xs text-emerald-200">
                Tracker rows parsed: {extraction.draft.testCases.length} tests,{" "}
                {extraction.draft.findings.length} findings.
              </div>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => setDraft({ draftId: extraction.draftId!, draft: extraction.draft! })}
              >
                Use parsed tracker
              </Button>
            </div>
          )}
          <div className="space-y-3 border-t border-slate-800 pt-4">
            <p className="text-xs font-medium uppercase tracking-wide text-slate-400">
              Use AI to structure the document
            </p>
            {providers.isLoading && <LoadingBlock label="Loading providers…" />}
            {providers.isError && <ErrorBlock message={providers.error.message} />}
            {providers.data && providers.data.length === 0 && (
              <ErrorBlock message="Configure an AI provider in Settings to use AI import. Spreadsheet trackers can be imported without AI." />
            )}
            {providers.data && providers.data.length > 0 && (
              <>
                <Field label="Provider">
                  <Select
                    value={selectedProviderId}
                    onValueChange={(value) => {
                      setProviderId(value);
                      setAllowCloud(false);
                    }}
                  >
                    {providers.data.map((provider) => (
                      <SelectItem key={provider.id} value={provider.id}>
                        {provider.name} · {provider.model}
                        {provider.isLocal ? " · local" : " · cloud"}
                      </SelectItem>
                    ))}
                  </Select>
                </Field>
                {selectedProvider && !selectedProvider.isLocal && (
                  <label className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[11px] text-amber-200/80">
                    <input
                      type="checkbox"
                      checked={allowCloud}
                      onChange={(event) => setAllowCloud(event.target.checked)}
                      className="mt-0.5 h-3.5 w-3.5 rounded border-slate-600 bg-slate-900"
                    />
                    <span>
                      This provider is not local. I confirm the extracted assessment text may be
                      sent to it. The redacted preview is shown before the call.
                    </span>
                  </label>
                )}
                <Field
                  label="Additional instructions"
                  hint="Optional; included in the redacted prompt."
                >
                  <Textarea
                    rows={3}
                    value={instructions}
                    onChange={(event) => setInstructions(event.target.value)}
                    placeholder="Focus areas, engagement conventions, or constraints."
                  />
                </Field>
                <div className="flex flex-wrap items-center gap-2 text-[11px] text-slate-500">
                  <Badge tone={selectedProvider?.isLocal ? "success" : "warning"}>
                    {selectedProvider?.isLocal ? "Local" : "Cloud"}
                  </Badge>
                  <span>
                    AI import returns a draft. You review the structured data before anything is
                    created.
                  </span>
                </div>
              </>
            )}
          </div>
          {error && <ErrorBlock message={error} />}
        </div>
      ) : (
        <div className="space-y-4">
          <Field
            label="Assessment file"
            hint="XLSX/XLSM tracker, DOCX, PDF, Markdown, JSON, or text up to 25 MB."
          >
            <input
              type="file"
              accept={ACCEPTED_FILES}
              onChange={(event) => {
                setFile(event.target.files?.[0] ?? null);
                setExtraction(null);
                setPreview(null);
                setDraft(null);
                setError(null);
              }}
              className="block w-full cursor-pointer rounded-lg border border-slate-700 bg-slate-950/60 px-3 py-2 text-xs text-slate-300 file:mr-3 file:rounded-md file:border-0 file:bg-slate-800 file:px-3 file:py-1.5 file:text-xs file:text-slate-200"
            />
          </Field>
          {file && (
            <p className="text-[11px] text-slate-500">
              {file.name} · {formatBytes(file.size)}
            </p>
          )}
          <div className="flex items-start gap-2 rounded-lg border border-slate-800 bg-slate-950/40 p-3 text-[11px] text-slate-500">
            <Sparkles className="mt-0.5 h-3.5 w-3.5 shrink-0 text-emerald-400" />
            <span>
              Spreadsheet trackers are parsed deterministically. Other formats use a configured AI
              provider, and every AI draft requires approval. The source file is not stored.
            </span>
          </div>
          {error && <ErrorBlock message={error} />}
        </div>
      )}
    </Dialog>
  );
}

function DraftReview({ draft, error }: { draft: ImportDraft; error: string | null }) {
  const sourceLabel = draft.sourceFormat === "ai" ? "AI-generated" : "Parsed tracker";
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <Badge tone={draft.sourceFormat === "ai" ? "warning" : "success"}>{sourceLabel}</Badge>
        <Badge>{draft.testCases.length} tests</Badge>
        <Badge>{draft.findings.length} findings</Badge>
        <Badge>{draft.scopeAssets.length} scope assets</Badge>
        <Badge>{draft.risks.length} risks</Badge>
      </div>
      <div className="rounded-lg border border-slate-800 bg-slate-950/40 p-3">
        <p className="text-sm font-medium text-slate-100">{draft.engagement.name}</p>
        <p className="mt-1 text-xs text-slate-400">
          {draft.engagement.clientCode || "No client code"}
          {draft.engagement.startDate && ` · ${draft.engagement.startDate}`}
          {draft.engagement.endDate && ` – ${draft.engagement.endDate}`}
        </p>
        {draft.engagement.applicationSummary && (
          <p className="mt-2 line-clamp-3 text-xs text-slate-400">
            {draft.engagement.applicationSummary}
          </p>
        )}
        {draft.engagement.attackSurface && (
          <p className="mt-2 line-clamp-3 text-xs text-slate-500">
            Attack surface: {draft.engagement.attackSurface}
          </p>
        )}
      </div>
      {draft.testCases.length > 0 && (
        <div className="overflow-hidden rounded-lg border border-slate-800">
          <div className="max-h-64 overflow-auto">
            <table className="w-full text-left text-xs">
              <thead className="sticky top-0 border-b border-slate-800 bg-slate-950 text-[10px] uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-3 py-2 font-medium">Test case</th>
                  <th className="px-3 py-2 font-medium">Status</th>
                  <th className="px-3 py-2 font-medium">Result</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800">
                {draft.testCases.slice(0, 100).map((testCase, index) => (
                  <tr key={`${testCase.title}-${index}`}>
                    <td className="px-3 py-2 text-slate-200">{testCase.title}</td>
                    <td className="px-3 py-2 text-slate-400">{testCase.status}</td>
                    <td className="max-w-[18rem] truncate px-3 py-2 text-slate-400">
                      {testCase.result || "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
      {draft.findings.length > 0 && (
        <div className="space-y-2">
          <p className="text-xs font-medium uppercase tracking-wide text-slate-400">Findings</p>
          {draft.findings.slice(0, 20).map((finding, index) => (
            <div
              key={`${finding.title}-${index}`}
              className="rounded-lg border border-slate-800 bg-slate-950/40 px-3 py-2"
            >
              <p className="text-xs font-medium text-slate-200">{finding.title}</p>
              <p className="mt-0.5 text-[11px] text-slate-500">
                Probability {finding.probability} · Impact {finding.impactScore}
                {finding.cwe && ` · ${finding.cwe}`}
                {finding.owasp && ` · ${finding.owasp}`}
              </p>
            </div>
          ))}
        </div>
      )}
      {draft.assumptions.length > 0 && (
        <div className="space-y-1">
          <p className="text-xs font-medium uppercase tracking-wide text-slate-400">Assumptions</p>
          <ul className="list-disc space-y-1 pl-5 text-xs text-slate-400">
            {draft.assumptions.slice(0, 20).map((assumption, index) => (
              <li key={`${assumption}-${index}`}>{assumption}</li>
            ))}
          </ul>
        </div>
      )}
      <p className="rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[11px] text-amber-200/80">
        Review names, dates, scope, results, and findings carefully. Imported data may need
        correction, and no source evidence is attached to this engagement.
      </p>
      {error && <ErrorBlock message={error} />}
    </div>
  );
}
