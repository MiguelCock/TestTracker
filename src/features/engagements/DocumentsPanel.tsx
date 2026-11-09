import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, FileOutput, FileSpreadsheet, FileText, Plus, Trash2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { Field } from "@/components/ui/input";
import { Select, SelectItem } from "@/components/ui/select";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import { base64ToBytes } from "@/lib/base64";
import { formatBytes, formatDateTime } from "@/lib/format";
import type { DocumentFormat, DocumentKind, GeneratedDocument } from "@/types";

const kindLabel: Record<DocumentKind, string> = {
  test_plan: "Test plan",
  test_case_tracker: "Test case tracker",
  findings_register: "Findings register",
  executive_summary: "Executive summary",
  technical_report: "Technical report",
};

const formatLabel: Record<DocumentFormat, string> = {
  pdf: "PDF",
  docx: "DOCX",
  xlsx: "XLSX",
};

const formatTone: Record<DocumentFormat, "danger" | "info" | "success"> = {
  pdf: "danger",
  docx: "info",
  xlsx: "success",
};

const kindTone: Record<DocumentKind, string> = {
  test_plan: "from-sky-500/20 to-cyan-500/5 text-sky-300",
  test_case_tracker: "from-emerald-500/20 to-teal-500/5 text-emerald-300",
  findings_register: "from-red-500/20 to-amber-500/5 text-red-300",
  executive_summary: "from-violet-500/20 to-fuchsia-500/5 text-violet-300",
  technical_report: "from-amber-500/20 to-orange-500/5 text-amber-300",
};

function supportsXlsx(kind: DocumentKind): boolean {
  return kind === "test_case_tracker" || kind === "findings_register";
}

export function DocumentsPanel({ engagementId }: { engagementId: string }) {
  const queryClient = useQueryClient();
  const [generateOpen, setGenerateOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);
  const documents = useQuery({
    queryKey: ["documents", engagementId],
    queryFn: () => api.listDocuments(engagementId),
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.deleteDocument(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["documents", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["audit", engagementId] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const download = async (item: GeneratedDocument) => {
    setError(null);
    setDownloading(item.id);
    try {
      const data = await api.getDocumentData(item.id);
      const mime =
        data.document.format === "pdf"
          ? "application/pdf"
          : data.document.format === "docx"
            ? "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            : "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
      const blob = new Blob([base64ToBytes(data.dataBase64)], { type: mime });
      const url = URL.createObjectURL(blob);
      const link = window.document.createElement("a");
      link.href = url;
      link.download = data.document.filename;
      window.document.body.appendChild(link);
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
          <h2 className="text-sm font-semibold text-white">Documents</h2>
          <p className="text-xs text-slate-500">
            Generate reports from the current engagement data. Files are encrypted at rest.
          </p>
        </div>
        <Button size="sm" onClick={() => setGenerateOpen(true)}>
          <Plus className="h-3.5 w-3.5" /> Generate document
        </Button>
      </div>

      <div className="rounded-xl border border-slate-800 bg-gradient-to-r from-emerald-500/10 via-sky-500/10 to-violet-500/10 p-5">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-emerald-300">
              Report workspace
            </p>
            <p className="mt-1 text-sm text-slate-200">
              {documents.data?.length ?? 0} generated document
              {(documents.data?.length ?? 0) === 1 ? "" : "s"} · encrypted at rest
            </p>
            <p className="mt-1 max-w-2xl text-xs text-slate-400">
              Generate a test plan, tracker, findings register, executive summary, or full technical
              report from the current engagement data.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Badge tone="success">PDF</Badge>
            <Badge tone="info">DOCX</Badge>
            <Badge tone="warning">XLSX</Badge>
          </div>
        </div>
      </div>

      {error && <ErrorBlock message={error} />}
      {documents.isLoading && <LoadingBlock label="Loading documents…" />}
      {documents.isError && <ErrorBlock message={documents.error.message} />}

      {documents.data && documents.data.length === 0 && (
        <EmptyState
          title="No documents generated"
          description="Create a test plan, test-case tracker, findings register, executive summary, or technical report."
          action={
            <Button size="sm" onClick={() => setGenerateOpen(true)}>
              <FileOutput className="h-3.5 w-3.5" /> Generate document
            </Button>
          }
        />
      )}

      {documents.data && documents.data.length > 0 && (
        <Card className="overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="border-b border-slate-800 bg-slate-950/40 text-[11px] uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Document</th>
                  <th className="px-4 py-3 font-medium">Format</th>
                  <th className="px-4 py-3 font-medium">Size</th>
                  <th className="px-4 py-3 font-medium">Generated</th>
                  <th className="px-4 py-3 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800">
                {documents.data.map((document) => (
                  <tr key={document.id} className="align-top">
                    <td className="max-w-md px-4 py-3">
                      <div className="flex items-start gap-2">
                        <span
                          className={`mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-gradient-to-br ring-1 ring-inset ring-white/5 ${kindTone[document.kind]}`}
                        >
                          {document.format === "xlsx" ? (
                            <FileSpreadsheet className="h-4 w-4" />
                          ) : (
                            <FileText className="h-4 w-4" />
                          )}
                        </span>
                        <div className="min-w-0">
                          <p className="truncate text-xs font-medium text-slate-200">
                            {document.title}
                          </p>
                          <p className="mt-0.5 text-[11px] text-slate-500">
                            {kindLabel[document.kind]} · {document.filename}
                          </p>
                        </div>
                      </div>
                    </td>
                    <td className="px-4 py-3">
                      <Badge tone={formatTone[document.format]}>
                        {formatLabel[document.format]}
                      </Badge>
                    </td>
                    <td className="px-4 py-3 text-xs text-slate-400">
                      {formatBytes(document.size)}
                    </td>
                    <td className="px-4 py-3 text-xs text-slate-500">
                      {formatDateTime(document.createdAt)}
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex justify-end gap-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label="Download document"
                          disabled={downloading === document.id}
                          onClick={() => void download(document)}
                        >
                          {downloading === document.id ? (
                            <Spinner />
                          ) : (
                            <Download className="h-3.5 w-3.5" />
                          )}
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label="Delete document"
                          onClick={() => {
                            if (window.confirm(`Delete "${document.filename}"?`)) {
                              remove.mutate(document.id);
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

      {generateOpen && (
        <GenerateDocumentDialog
          open={generateOpen}
          onOpenChange={setGenerateOpen}
          engagementId={engagementId}
          onGenerated={() => {
            void queryClient.invalidateQueries({ queryKey: ["documents", engagementId] });
            void queryClient.invalidateQueries({ queryKey: ["audit", engagementId] });
          }}
        />
      )}
    </div>
  );
}

function GenerateDocumentDialog({
  open,
  onOpenChange,
  engagementId,
  onGenerated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  engagementId: string;
  onGenerated: () => void;
}) {
  const [kind, setKind] = useState<DocumentKind>("technical_report");
  const [format, setFormat] = useState<DocumentFormat>("pdf");
  const [error, setError] = useState<string | null>(null);
  const generate = useMutation({
    mutationFn: () => api.generateDocument(engagementId, kind, format),
    onSuccess: () => {
      onGenerated();
      onOpenChange(false);
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setError(null);
        onOpenChange(next);
      }}
      title="Generate document"
      description="The document is rendered from the current engagement, tests, findings, and evidence metadata."
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={() => generate.mutate()} disabled={generate.isPending}>
            {generate.isPending && <Spinner />}
            Generate
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <Field label="Document">
          <Select
            value={kind}
            onValueChange={(value) => {
              const next = value as DocumentKind;
              setKind(next);
              if (!supportsXlsx(next) && format === "xlsx") {
                setFormat("pdf");
              }
            }}
          >
            {(Object.keys(kindLabel) as DocumentKind[]).map((value) => (
              <SelectItem key={value} value={value}>
                {kindLabel[value]}
              </SelectItem>
            ))}
          </Select>
        </Field>
        <Field
          label="Format"
          hint={
            supportsXlsx(kind)
              ? "XLSX is available for the tracker and findings register."
              : "PDF and DOCX are available for this document type."
          }
        >
          <Select value={format} onValueChange={(value) => setFormat(value as DocumentFormat)}>
            <SelectItem value="pdf">PDF</SelectItem>
            <SelectItem value="docx">DOCX</SelectItem>
            {supportsXlsx(kind) && <SelectItem value="xlsx">XLSX</SelectItem>}
          </Select>
        </Field>
        {error && <ErrorBlock message={error} />}
      </div>
    </Dialog>
  );
}
