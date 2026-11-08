import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, FileUp, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Field, Input } from "@/components/ui/input";
import { ErrorBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import { readFileAsBase64 } from "@/lib/base64";
import { formatBytes } from "@/lib/format";
import type { BundleImportResult } from "@/types";

const MAX_BUNDLE_BYTES = 40 * 1024 * 1024;

export function ImportBundleDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [file, setFile] = useState<File | null>(null);
  const [passphrase, setPassphrase] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<BundleImportResult | null>(null);

  const importBundle = useMutation({
    mutationFn: async () => {
      if (!file) throw new Error("Choose a .ttbundle file.");
      if (file.size > MAX_BUNDLE_BYTES) {
        throw new Error("Bundle files must be 40 MB or smaller.");
      }
      const data = await readFileAsBase64(file);
      return api.importBundle(data, passphrase);
    },
    onSuccess: (data) => {
      setResult(data);
      void queryClient.invalidateQueries({ queryKey: ["engagements"] });
      void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const close = () => {
    onOpenChange(false);
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setError(null);
        onOpenChange(next);
      }}
      title="Import engagement bundle"
      description="Imports the bundle as a new engagement with new identifiers. Existing data is not modified."
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
        ) : (
          <>
            <Button variant="secondary" onClick={close}>
              Cancel
            </Button>
            <Button onClick={() => importBundle.mutate()} disabled={importBundle.isPending}>
              {importBundle.isPending ? <Spinner /> : <FileUp className="h-4 w-4" />}
              Import bundle
            </Button>
          </>
        )
      }
    >
      {result ? (
        <div className="space-y-4">
          <div className="flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-xs text-emerald-300">
            <CheckCircle2 className="h-4 w-4" /> Bundle imported successfully.
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
            <div>
              <dt className="text-slate-500">Evidence</dt>
              <dd className="mt-0.5 text-slate-200">{result.evidenceCount} file(s)</dd>
            </div>
            <div>
              <dt className="text-slate-500">Documents</dt>
              <dd className="mt-0.5 text-slate-200">{result.documentCount} file(s)</dd>
            </div>
          </dl>
          <p className="rounded-lg border border-slate-800 bg-slate-950/40 p-3 text-[11px] text-slate-500">
            Sender: {result.senderDisplayName || "Unknown"} · fingerprint{" "}
            <span className="font-mono text-slate-400">{result.senderFingerprint}</span>
          </p>
          <p className="text-[11px] text-slate-500">
            The signature verifies bundle integrity but does not prove the sender's identity.
            Confirm the fingerprint out of band before trusting the contents.
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          <Field label="Bundle file" hint="Select a .ttbundle file up to 40 MB.">
            <input
              type="file"
              accept=".ttbundle"
              onChange={(event) => {
                setFile(event.target.files?.[0] ?? null);
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
          <Field label="Bundle passphrase">
            <Input
              type="password"
              autoComplete="off"
              value={passphrase}
              onChange={(event) => setPassphrase(event.target.value)}
            />
          </Field>
          <div className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[11px] text-amber-200/80">
            <ShieldCheck className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            Import creates a new engagement. Snapshots, approvals, audit events, and AI data are not
            included in bundles.
          </div>
          {error && <ErrorBlock message={error} />}
        </div>
      )}
    </Dialog>
  );
}
