import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { CheckCircle2, Download, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Field, Input } from "@/components/ui/input";
import { ErrorBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import { downloadBase64File } from "@/lib/download";
import { formatBytes } from "@/lib/format";
import type { BundleExportResult } from "@/types";

export function ExportBundleDialog({
  open,
  onOpenChange,
  engagementId,
  engagementName,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  engagementId: string;
  engagementName: string;
}) {
  const [passphrase, setPassphrase] = useState("");
  const [confirm, setConfirm] = useState("");
  const [includeDocuments, setIncludeDocuments] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<BundleExportResult | null>(null);

  const exportBundle = useMutation({
    mutationFn: () => api.exportBundle(engagementId, passphrase, includeDocuments),
    onSuccess: (data) => {
      setResult(data);
      downloadBase64File(data.filename, data.dataBase64, "application/octet-stream");
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const submit = () => {
    setError(null);
    if (passphrase.length < 12) {
      setError("Use a bundle passphrase of at least 12 characters.");
      return;
    }
    if (passphrase !== confirm) {
      setError("Passphrases do not match.");
      return;
    }
    exportBundle.mutate();
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setError(null);
        onOpenChange(next);
      }}
      title="Export engagement bundle"
      description="Creates a passphrase-encrypted .ttbundle containing the engagement, findings, evidence, and optionally generated documents."
      footer={
        result ? (
          <>
            <Button variant="secondary" onClick={() => onOpenChange(false)}>
              Close
            </Button>
            <Button
              onClick={() =>
                downloadBase64File(result.filename, result.dataBase64, "application/octet-stream")
              }
            >
              <Download className="h-4 w-4" /> Download again
            </Button>
          </>
        ) : (
          <>
            <Button variant="secondary" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button onClick={submit} disabled={exportBundle.isPending}>
              {exportBundle.isPending ? <Spinner /> : <Download className="h-4 w-4" />}
              Export bundle
            </Button>
          </>
        )
      }
    >
      {result ? (
        <div className="space-y-4">
          <div className="flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-xs text-emerald-300">
            <CheckCircle2 className="h-4 w-4" /> Bundle exported and downloaded.
          </div>
          <dl className="grid grid-cols-2 gap-3 text-xs">
            <div>
              <dt className="text-slate-500">Engagement</dt>
              <dd className="mt-0.5 text-slate-200">{result.engagementName}</dd>
            </div>
            <div>
              <dt className="text-slate-500">File</dt>
              <dd className="mt-0.5 break-all text-slate-200">{result.filename}</dd>
            </div>
            <div>
              <dt className="text-slate-500">Size</dt>
              <dd className="mt-0.5 text-slate-200">{formatBytes(result.size)}</dd>
            </div>
            <div>
              <dt className="text-slate-500">Contents</dt>
              <dd className="mt-0.5 text-slate-200">
                {result.evidenceCount} evidence file(s), {result.documentCount} document(s)
              </dd>
            </div>
          </dl>
          <p className="rounded-lg border border-slate-800 bg-slate-950/40 p-3 text-[11px] text-slate-500">
            Sender signature fingerprint:{" "}
            <span className="font-mono text-slate-400">{result.signatureFingerprint}</span>
          </p>
          <div className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[11px] text-amber-200/80">
            <ShieldCheck className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            The bundle contains sensitive client data. Store and share it only through approved
            channels. The passphrase cannot be recovered.
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          <Field label="Bundle passphrase" hint="At least 12 characters. Required to import.">
            <Input
              type="password"
              autoFocus
              autoComplete="new-password"
              value={passphrase}
              onChange={(event) => setPassphrase(event.target.value)}
            />
          </Field>
          <Field label="Confirm passphrase">
            <Input
              type="password"
              autoComplete="new-password"
              value={confirm}
              onChange={(event) => setConfirm(event.target.value)}
            />
          </Field>
          <label className="flex items-center gap-2 text-xs text-slate-300">
            <input
              type="checkbox"
              className="h-4 w-4 rounded border-slate-600 bg-slate-900"
              checked={includeDocuments}
              onChange={(event) => setIncludeDocuments(event.target.checked)}
            />
            Include generated documents
          </label>
          <p className="text-[11px] text-slate-500">
            Exporting {engagementName}. The bundle includes scope, risks, assumptions, contacts,
            tests, findings, and evidence.
          </p>
          {error && <ErrorBlock message={error} />}
        </div>
      )}
    </Dialog>
  );
}
