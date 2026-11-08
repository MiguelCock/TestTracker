import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Download,
  Eye,
  File as FileIcon,
  FileImage,
  FileText,
  Paperclip,
  Plus,
  Trash2,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { Field, Select, Textarea } from "@/components/ui/input";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import { base64ToBytes, readFileAsBase64 } from "@/lib/base64";
import { formatBytes, formatDateTime } from "@/lib/format";
import type { Evidence, Finding, TestCase } from "@/types";

const MAX_BYTES = 8 * 1024 * 1024;

export function EvidencePanel({ engagementId }: { engagementId: string }) {
  const queryClient = useQueryClient();
  const [addOpen, setAddOpen] = useState(false);
  const [preview, setPreview] = useState<Evidence | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);

  const evidence = useQuery({
    queryKey: ["evidence", engagementId],
    queryFn: () => api.listEvidence(engagementId),
  });
  const findings = useQuery({
    queryKey: ["findings", engagementId],
    queryFn: () => api.listFindings(engagementId),
  });
  const testCases = useQuery({
    queryKey: ["test-cases", engagementId],
    queryFn: () => api.listTestCases(engagementId),
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.deleteEvidence(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["evidence", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["engagement", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["engagements"] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const download = async (item: Evidence) => {
    setError(null);
    setDownloading(item.id);
    try {
      const data = await api.getEvidenceData(item.id);
      const bytes = base64ToBytes(data.dataBase64);
      const blob = new Blob([bytes], { type: data.evidence.mimeType });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = data.evidence.filename;
      document.body.appendChild(link);
      link.click();
      link.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 1_000);
    } catch (downloadError) {
      setError(downloadError instanceof Error ? downloadError.message : "Download failed.");
    } finally {
      setDownloading(null);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-white">Evidence</h2>
          <p className="text-xs text-slate-500">
            Encrypted files, screenshots, and notes attached to this engagement.
          </p>
        </div>
        <Button size="sm" onClick={() => setAddOpen(true)}>
          <Plus className="h-3.5 w-3.5" /> Add evidence
        </Button>
      </div>

      {error && <ErrorBlock message={error} />}
      {evidence.isLoading && <LoadingBlock label="Loading evidence…" />}
      {evidence.isError && <ErrorBlock message={evidence.error.message} />}
      {remove.isPending && <p className="text-xs text-slate-500">Deleting evidence…</p>}

      {evidence.data && evidence.data.length === 0 && (
        <EmptyState
          title="No evidence attached"
          description="Attach screenshots, request/response notes, or files. Content is encrypted before it touches disk."
          action={
            <Button size="sm" onClick={() => setAddOpen(true)}>
              <Paperclip className="h-3.5 w-3.5" /> Add evidence
            </Button>
          }
        />
      )}

      {evidence.data && evidence.data.length > 0 && (
        <Card className="overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="border-b border-slate-800 bg-slate-950/40 text-[11px] uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-3 font-medium">File</th>
                  <th className="px-4 py-3 font-medium">Links</th>
                  <th className="px-4 py-3 font-medium">Size</th>
                  <th className="px-4 py-3 font-medium">Added</th>
                  <th className="px-4 py-3 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800">
                {evidence.data.map((item) => {
                  const finding = findings.data?.find((entry) => entry.id === item.findingId);
                  const testCase = testCases.data?.find((entry) => entry.id === item.testCaseId);
                  return (
                    <tr key={item.id} className="align-top transition-colors hover:bg-slate-800/20">
                      <td className="max-w-md px-4 py-3">
                        <div className="flex items-start gap-2">
                          <span className="mt-0.5 text-slate-500">{iconFor(item.mimeType)}</span>
                          <div className="min-w-0">
                            <p className="truncate text-xs font-medium text-slate-200">
                              {item.filename}
                            </p>
                            {item.caption && (
                              <p className="mt-0.5 text-[11px] text-slate-500">{item.caption}</p>
                            )}
                            <Badge className="mt-1">{item.mimeType}</Badge>
                          </div>
                        </div>
                      </td>
                      <td className="px-4 py-3 text-[11px] text-slate-400">
                        {finding && (
                          <p>
                            {finding.code} · {finding.title}
                          </p>
                        )}
                        {testCase && <p className="mt-0.5">{testCase.title}</p>}
                        {!finding && !testCase && <span className="text-slate-600">—</span>}
                      </td>
                      <td className="px-4 py-3 text-xs text-slate-400">{formatBytes(item.size)}</td>
                      <td className="px-4 py-3 text-xs text-slate-500">
                        {formatDateTime(item.createdAt)}
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex justify-end gap-1">
                          {previewable(item.mimeType) && (
                            <Button
                              variant="ghost"
                              size="icon"
                              aria-label="Preview evidence"
                              onClick={() => setPreview(item)}
                            >
                              <Eye className="h-3.5 w-3.5" />
                            </Button>
                          )}
                          <Button
                            variant="ghost"
                            size="icon"
                            aria-label="Download evidence"
                            disabled={downloading === item.id}
                            onClick={() => void download(item)}
                          >
                            {downloading === item.id ? (
                              <Spinner />
                            ) : (
                              <Download className="h-3.5 w-3.5" />
                            )}
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            aria-label="Delete evidence"
                            onClick={() => {
                              if (window.confirm(`Delete "${item.filename}"?`)) {
                                remove.mutate(item.id);
                              }
                            }}
                          >
                            <Trash2 className="h-3.5 w-3.5 text-slate-500" />
                          </Button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </Card>
      )}

      <AddEvidenceDialog
        open={addOpen}
        onOpenChange={setAddOpen}
        engagementId={engagementId}
        findings={findings.data ?? []}
        testCases={testCases.data ?? []}
        onSaved={() => {
          void queryClient.invalidateQueries({ queryKey: ["evidence", engagementId] });
          void queryClient.invalidateQueries({ queryKey: ["engagement", engagementId] });
        }}
      />

      <PreviewDialog preview={preview} onClose={() => setPreview(null)} onDownload={download} />
    </div>
  );
}

function AddEvidenceDialog({
  open,
  onOpenChange,
  engagementId,
  findings,
  testCases,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  engagementId: string;
  findings: Finding[];
  testCases: TestCase[];
  onSaved: () => void;
}) {
  const [file, setFile] = useState<File | null>(null);
  const [caption, setCaption] = useState("");
  const [findingId, setFindingId] = useState("");
  const [testCaseId, setTestCaseId] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [reading, setReading] = useState(false);

  const add = useMutation({
    mutationFn: api.addEvidence,
    onSuccess: () => {
      setFile(null);
      setCaption("");
      setFindingId("");
      setTestCaseId("");
      onSaved();
      onOpenChange(false);
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const submit = async () => {
    setError(null);
    if (!file) {
      setError("Choose a file to attach.");
      return;
    }
    if (file.size > MAX_BYTES) {
      setError("Evidence files must be 8 MB or smaller.");
      return;
    }
    setReading(true);
    try {
      const dataBase64 = await readFileAsBase64(file);
      add.mutate({
        engagementId,
        findingId: findingId || null,
        testCaseId: testCaseId || null,
        filename: file.name,
        mimeType: file.type || "application/octet-stream",
        caption: caption.trim(),
        dataBase64,
      });
    } catch (readError) {
      setError(readError instanceof Error ? readError.message : "Unable to read the file.");
    } finally {
      setReading(false);
    }
  };

  const busy = reading || add.isPending;

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setError(null);
        onOpenChange(next);
      }}
      title="Add evidence"
      description="The file is encrypted with this engagement's key before it is written to disk."
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={() => void submit()} disabled={busy}>
            {busy && <Spinner />}
            Attach evidence
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <Field label="File" hint="Up to 8 MB. Screenshots and text notes are ideal.">
          <input
            type="file"
            onChange={(event) => {
              const selected = event.target.files?.[0] ?? null;
              setFile(selected);
              setError(null);
            }}
            className="block w-full cursor-pointer rounded-lg border border-slate-700 bg-slate-950/60 px-3 py-2 text-xs text-slate-300 file:mr-3 file:rounded-md file:border-0 file:bg-slate-800 file:px-3 file:py-1.5 file:text-xs file:text-slate-200"
          />
        </Field>
        <Field label="Caption">
          <Textarea
            rows={2}
            placeholder="What does this evidence show?"
            value={caption}
            onChange={(event) => setCaption(event.target.value)}
          />
        </Field>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="Link finding">
            <Select value={findingId} onChange={(event) => setFindingId(event.target.value)}>
              <option value="">No finding</option>
              {findings.map((finding) => (
                <option key={finding.id} value={finding.id}>
                  {finding.code} · {finding.title}
                </option>
              ))}
            </Select>
          </Field>
          <Field label="Link test case">
            <Select value={testCaseId} onChange={(event) => setTestCaseId(event.target.value)}>
              <option value="">No test case</option>
              {testCases.map((testCase) => (
                <option key={testCase.id} value={testCase.id}>
                  {testCase.title}
                </option>
              ))}
            </Select>
          </Field>
        </div>
        {error && <ErrorBlock message={error} />}
      </div>
    </Dialog>
  );
}

function PreviewDialog({
  preview,
  onClose,
  onDownload,
}: {
  preview: Evidence | null;
  onClose: () => void;
  onDownload: (evidence: Evidence) => Promise<void>;
}) {
  const data = useQuery({
    queryKey: ["evidence-data", preview?.id],
    queryFn: () => api.getEvidenceData(preview!.id),
    enabled: Boolean(preview),
  });

  return (
    <Dialog
      open={preview !== null}
      onOpenChange={(next) => !next && onClose()}
      title={preview?.filename ?? "Evidence"}
      description={preview?.caption || undefined}
      wide
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Close
          </Button>
          {preview && (
            <Button onClick={() => void onDownload(preview)}>
              <Download className="h-4 w-4" /> Download
            </Button>
          )}
        </>
      }
    >
      {data.isLoading && <LoadingBlock label="Decrypting evidence…" />}
      {data.isError && <ErrorBlock message={data.error.message} />}
      {data.data && <EvidencePreview data={data.data} />}
    </Dialog>
  );
}

function EvidencePreview({ data }: { data: { evidence: Evidence; dataBase64: string } }) {
  if (data.evidence.mimeType.startsWith("image/")) {
    return (
      <img
        src={`data:${data.evidence.mimeType};base64,${data.dataBase64}`}
        alt={data.evidence.caption || data.evidence.filename}
        className="mx-auto max-h-[60vh] rounded-lg border border-slate-800"
      />
    );
  }
  if (isText(data.evidence.mimeType)) {
    return (
      <pre className="max-h-[60vh] overflow-auto whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs text-slate-300">
        {decodeText(data.dataBase64)}
      </pre>
    );
  }
  return (
    <p className="py-8 text-center text-xs text-slate-500">
      Preview is not available for this file type. Download it to view the contents.
    </p>
  );
}

function decodeText(base64: string): string {
  return new TextDecoder().decode(base64ToBytes(base64));
}

function previewable(mimeType: string): boolean {
  return mimeType.startsWith("image/") || isText(mimeType);
}

function isText(mimeType: string): boolean {
  return (
    mimeType.startsWith("text/") ||
    mimeType.includes("json") ||
    mimeType.includes("xml") ||
    mimeType.includes("csv") ||
    mimeType === "application/x-www-form-urlencoded"
  );
}

function iconFor(mimeType: string) {
  if (mimeType.startsWith("image/")) return <FileImage className="h-4 w-4" />;
  if (isText(mimeType)) return <FileText className="h-4 w-4" />;
  return <FileIcon className="h-4 w-4" />;
}
