import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  BadgeCheck,
  BadgeX,
  FileClock,
  FileSearch,
  PenLine,
  Plus,
  ShieldCheck,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { Field, Input, Select, Textarea } from "@/components/ui/input";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import { useSession } from "@/hooks/useSession";
import { formatDateTime, truncate } from "@/lib/format";
import type { ApprovalDecision, SnapshotSummary } from "@/types";

const decisionLabel: Record<ApprovalDecision, string> = {
  approved: "Approved",
  changes_requested: "Changes requested",
  rejected: "Rejected",
};

const decisionTone: Record<ApprovalDecision, "success" | "warning" | "danger"> = {
  approved: "success",
  changes_requested: "warning",
  rejected: "danger",
};

export function ReviewPanel({ engagementId }: { engagementId: string }) {
  const queryClient = useQueryClient();
  const session = useSession();
  const role = session.data?.profile?.role;
  const canWrite = role !== "reviewer";
  const canApprove = role !== "tester";
  const [snapshotOpen, setSnapshotOpen] = useState(false);
  const [signOffOpen, setSignOffOpen] = useState(false);
  const [viewing, setViewing] = useState<string | null>(null);

  const snapshots = useQuery({
    queryKey: ["snapshots", engagementId],
    queryFn: () => api.listSnapshots(engagementId),
  });
  const approvals = useQuery({
    queryKey: ["approvals", engagementId],
    queryFn: () => api.listApprovals(engagementId),
  });

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["snapshots", engagementId] });
    void queryClient.invalidateQueries({ queryKey: ["approvals", engagementId] });
    void queryClient.invalidateQueries({ queryKey: ["audit", engagementId] });
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-white">Review and sign-off</h2>
          <p className="text-xs text-slate-500">
            Immutable snapshots and Ed25519-signed reviewer decisions.
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setSnapshotOpen(true)}
            disabled={!canWrite}
          >
            <FileClock className="h-3.5 w-3.5" /> Create snapshot
          </Button>
          <Button
            size="sm"
            onClick={() => setSignOffOpen(true)}
            disabled={!canApprove || !snapshots.data || snapshots.data.length === 0}
          >
            <PenLine className="h-3.5 w-3.5" /> Sign off
          </Button>
        </div>
      </div>

      {(snapshots.isError || approvals.isError) && (
        <ErrorBlock message={(snapshots.error ?? approvals.error)!.message} />
      )}

      <div className="grid gap-4 xl:grid-cols-2">
        <Card>
          <CardHeader
            title="Snapshots"
            description="Point-in-time copies of the engagement record."
          />
          <CardBody className="p-0">
            {snapshots.isLoading && <LoadingBlock label="Loading snapshots…" />}
            {snapshots.data && snapshots.data.length === 0 && (
              <div className="p-5">
                <EmptyState
                  title="No snapshots"
                  description="Create a snapshot before requesting review or sign-off."
                  action={
                    <Button size="sm" onClick={() => setSnapshotOpen(true)}>
                      <Plus className="h-3.5 w-3.5" /> Create snapshot
                    </Button>
                  }
                />
              </div>
            )}
            {snapshots.data && snapshots.data.length > 0 && (
              <ul className="divide-y divide-slate-800">
                {snapshots.data.map((snapshot) => (
                  <li
                    key={snapshot.id}
                    className="flex items-center justify-between gap-3 px-5 py-3"
                  >
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <p className="text-xs font-medium text-slate-200">
                          v{snapshot.version} · {snapshot.label || "Untitled snapshot"}
                        </p>
                        {snapshot.verified ? (
                          <Badge tone="success">
                            <BadgeCheck className="mr-1 h-3 w-3" /> Hash verified
                          </Badge>
                        ) : (
                          <Badge tone="danger">
                            <BadgeX className="mr-1 h-3 w-3" /> Hash mismatch
                          </Badge>
                        )}
                      </div>
                      <p className="mt-0.5 text-[11px] text-slate-500">
                        {formatDateTime(snapshot.createdAt)} ·{" "}
                        <span className="font-mono">{truncate(snapshot.contentHash, 18)}</span>
                      </p>
                    </div>
                    <Button variant="ghost" size="sm" onClick={() => setViewing(snapshot.id)}>
                      <FileSearch className="h-3.5 w-3.5" /> View
                    </Button>
                  </li>
                ))}
              </ul>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader
            title="Approvals"
            description="Signatures are verified locally against the snapshot hash."
          />
          <CardBody className="p-0">
            {approvals.isLoading && <LoadingBlock label="Loading approvals…" />}
            {approvals.data && approvals.data.length === 0 && (
              <div className="p-5">
                <EmptyState
                  title="No sign-offs"
                  description="A reviewer can approve, request changes, or reject the latest snapshot."
                />
              </div>
            )}
            {approvals.data && approvals.data.length > 0 && (
              <ul className="divide-y divide-slate-800">
                {approvals.data.map((approval) => (
                  <li key={approval.id} className="px-5 py-3">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-xs font-medium text-slate-200">
                        {approval.reviewerName}
                      </span>
                      <Badge tone={decisionTone[approval.decision]}>
                        {decisionLabel[approval.decision]}
                      </Badge>
                      {approval.verified ? (
                        <Badge tone="success">
                          <BadgeCheck className="mr-1 h-3 w-3" /> Verified
                        </Badge>
                      ) : (
                        <Badge tone="danger">
                          <BadgeX className="mr-1 h-3 w-3" /> Unverified
                        </Badge>
                      )}
                      <span className="ml-auto text-[11px] text-slate-500">
                        {formatDateTime(approval.signedAt)}
                      </span>
                    </div>
                    {approval.comment && (
                      <p className="mt-1 text-xs text-slate-400">{approval.comment}</p>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </CardBody>
        </Card>
      </div>

      <div className="flex items-start gap-2 rounded-lg border border-slate-800 bg-slate-900/40 p-3 text-[11px] text-slate-500">
        <ShieldCheck className="mt-0.5 h-3.5 w-3.5 shrink-0 text-emerald-500/70" />A snapshot is
        immutable. Sign-offs cover the snapshot hash, decision, reviewer, and timestamp; editing the
        database invalidates the signature.
      </div>

      {snapshotOpen && (
        <CreateSnapshotDialog
          open={snapshotOpen}
          onOpenChange={setSnapshotOpen}
          engagementId={engagementId}
          onSaved={invalidate}
        />
      )}
      {signOffOpen && (
        <SignOffDialog
          open={signOffOpen}
          onOpenChange={setSignOffOpen}
          engagementId={engagementId}
          snapshots={snapshots.data ?? []}
          onSaved={invalidate}
        />
      )}
      <ViewSnapshotDialog snapshotId={viewing} onClose={() => setViewing(null)} />
    </div>
  );
}

function CreateSnapshotDialog({
  open,
  onOpenChange,
  engagementId,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  engagementId: string;
  onSaved: () => void;
}) {
  const [label, setLabel] = useState("");
  const [error, setError] = useState<string | null>(null);
  const create = useMutation({
    mutationFn: () => api.createSnapshot(engagementId, label.trim()),
    onSuccess: () => {
      setLabel("");
      onSaved();
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
      title="Create snapshot"
      description="Captures scope, risks, tests, findings, and evidence metadata at this version."
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={() => create.mutate()} disabled={create.isPending}>
            {create.isPending && <Spinner />}
            Create snapshot
          </Button>
        </>
      }
    >
      <Field label="Label" hint="Optional, for example “Ready for peer review”.">
        <Input
          autoFocus
          value={label}
          maxLength={200}
          onChange={(event) => setLabel(event.target.value)}
          placeholder="Ready for peer review"
        />
      </Field>
      {error && (
        <div className="mt-3">
          <ErrorBlock message={error} />
        </div>
      )}
    </Dialog>
  );
}

function SignOffDialog({
  open,
  onOpenChange,
  engagementId,
  snapshots,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  engagementId: string;
  snapshots: SnapshotSummary[];
  onSaved: () => void;
}) {
  const [snapshotId, setSnapshotId] = useState(snapshots[0]?.id ?? "");
  const [decision, setDecision] = useState<ApprovalDecision>("approved");
  const [comment, setComment] = useState("");
  const [error, setError] = useState<string | null>(null);

  const sign = useMutation({
    mutationFn: () => api.signOff(engagementId, snapshotId, decision, comment.trim()),
    onSuccess: () => {
      onSaved();
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
      title="Sign off"
      description="Your profile's Ed25519 key signs the snapshot hash and decision."
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={() => sign.mutate()} disabled={sign.isPending || !snapshotId}>
            {sign.isPending && <Spinner />}
            Sign decision
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <Field label="Snapshot">
          <Select value={snapshotId} onChange={(event) => setSnapshotId(event.target.value)}>
            {snapshots.map((snapshot) => (
              <option key={snapshot.id} value={snapshot.id}>
                v{snapshot.version} · {snapshot.label || "Untitled"} ·{" "}
                {truncate(snapshot.contentHash, 10)}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="Decision">
          <Select
            value={decision}
            onChange={(event) => setDecision(event.target.value as ApprovalDecision)}
          >
            <option value="approved">Approved</option>
            <option value="changes_requested">Changes requested</option>
            <option value="rejected">Rejected</option>
          </Select>
        </Field>
        <Field label="Comment">
          <Textarea
            rows={3}
            value={comment}
            onChange={(event) => setComment(event.target.value)}
            placeholder="Optional reviewer notes"
          />
        </Field>
        {error && <ErrorBlock message={error} />}
      </div>
    </Dialog>
  );
}

function ViewSnapshotDialog({
  snapshotId,
  onClose,
}: {
  snapshotId: string | null;
  onClose: () => void;
}) {
  const snapshot = useQuery({
    queryKey: ["snapshot", snapshotId],
    queryFn: () => api.getSnapshot(snapshotId!),
    enabled: Boolean(snapshotId),
  });

  return (
    <Dialog
      open={snapshotId !== null}
      onOpenChange={(next) => !next && onClose()}
      title={
        snapshot.data
          ? `Snapshot v${snapshot.data.version} · ${snapshot.data.label || "Untitled"}`
          : "Snapshot"
      }
      description={
        snapshot.data ? `Hash ${snapshot.data.contentHash}` : "Loading snapshot content…"
      }
      wide
      footer={
        <Button variant="secondary" onClick={onClose}>
          Close
        </Button>
      }
    >
      {snapshot.isLoading && <LoadingBlock label="Loading snapshot…" />}
      {snapshot.isError && <ErrorBlock message={snapshot.error.message} />}
      {snapshot.data && (
        <pre className="max-h-[60vh] overflow-auto whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-950 p-4 text-[11px] leading-relaxed text-slate-300">
          {pretty(snapshot.data.snapshotJson)}
        </pre>
      )}
    </Dialog>
  );
}

function pretty(value: string): string {
  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}
