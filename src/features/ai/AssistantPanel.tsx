import { useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  FileText,
  Image,
  Lock,
  Send,
  Settings2,
  ShieldCheck,
  Sparkles,
  Target,
  Trash2,
  Upload,
  X,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { Field, Textarea } from "@/components/ui/input";
import { Select, SelectItem } from "@/components/ui/select";
import { ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { useSession } from "@/hooks/useSession";
import { api } from "@/lib/api";
import { readFileAsBase64 } from "@/lib/base64";
import { formatBytes } from "@/lib/format";
import type {
  AgentProposal,
  AgentSkill,
  AiPolicy,
  AssistantFocus,
  AssistantPreview,
  AssistantStep,
  AssistantToolRequest,
  AssistantTurn,
  EngagementFile,
  ToolPermission,
} from "@/types";

const policyLabel: Record<AiPolicy, string> = {
  off: "Off",
  local_only: "Local models only",
  approved_cloud: "Approved cloud providers",
};

const skillLabel: Record<AgentSkill, string> = {
  application_summary: "Application summary",
  attack_surface: "Attack surface",
  assumptions: "Assumptions",
  test_cases: "Test cases",
  test_approach: "Test approach",
  finding_draft: "Finding draft",
  finding_improve: "Improve finding",
};

type AssistantMessage = {
  role: "user" | "assistant";
  content: string;
  note?: string;
  proposal?: AgentProposal;
  applied?: boolean;
};

export function AssistantPanel({
  engagementId,
  focus,
  onFocusChange,
}: {
  engagementId: string;
  focus: AssistantFocus;
  onFocusChange: (focus: AssistantFocus) => void;
}) {
  const queryClient = useQueryClient();
  const session = useSession();
  const providers = useQuery({ queryKey: ["ai-providers"], queryFn: api.listAiProviders });
  const policy = useQuery({
    queryKey: ["ai-policy", engagementId],
    queryFn: () => api.getAiPolicy(engagementId),
  });
  const [providerId, setProviderId] = useState("");
  const [prompt, setPrompt] = useState("");
  const [preview, setPreview] = useState<AssistantPreview | null>(null);
  const [previewMessage, setPreviewMessage] = useState("");
  const [toolRequest, setToolRequest] = useState<AssistantToolRequest | null>(null);
  const [fileIds, setFileIds] = useState<Set<string>>(new Set());
  const [permissionsOpen, setPermissionsOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const files = useQuery({
    queryKey: ["engagement-files", engagementId],
    queryFn: () => api.listEngagementFiles(engagementId),
  });
  const permissions = useQuery({
    queryKey: ["tool-permissions", engagementId],
    queryFn: () => api.listToolPermissions(engagementId),
    enabled: permissionsOpen,
  });
  const uploadFile = useMutation({
    mutationFn: async (file: File) => {
      if (file.size > 25 * 1024 * 1024) {
        throw new Error("Reference files must be 25 MB or smaller.");
      }
      const data = await readFileAsBase64(file);
      return api.addEngagementFile(engagementId, file.name, file.type, data);
    },
    onSuccess: (file) => {
      setError(null);
      setFileIds((current) => new Set(current).add(file.id));
      void queryClient.invalidateQueries({ queryKey: ["engagement-files", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["audit", engagementId] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });
  const deleteFile = useMutation({
    mutationFn: (id: string) => api.deleteEngagementFile(id),
    onSuccess: (_result, id) => {
      setFileIds((current) => {
        const next = new Set(current);
        next.delete(id);
        return next;
      });
      void queryClient.invalidateQueries({ queryKey: ["engagement-files", engagementId] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });
  const revokePermission = useMutation({
    mutationFn: (tool: string) => api.revokeToolPermission(engagementId, tool),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tool-permissions", engagementId] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });
  const conversation = useQuery<AssistantMessage[]>({
    queryKey: ["assistant-conversation", engagementId],
    queryFn: () => [],
    staleTime: Infinity,
    gcTime: Infinity,
  });
  const messages = conversation.data ?? [];
  const setMessages = (updater: (current: AssistantMessage[]) => AssistantMessage[]) => {
    queryClient.setQueryData<AssistantMessage[]>(
      ["assistant-conversation", engagementId],
      (current) => updater(current ?? []),
    );
  };

  const selectedProviderId = providerId || providers.data?.[0]?.id || "";
  const selectedProvider = providers.data?.find((provider) => provider.id === selectedProviderId);
  const activePolicy = policy.data ?? "local_only";
  const role = session.data?.profile?.role;
  const canManage = role === "admin" || role === "lead";
  const canUseAi = role === "admin" || role === "lead" || role === "tester";
  const policyAllows = selectedProvider
    ? activePolicy === "approved_cloud" ||
      (activePolicy === "local_only" && selectedProvider.isLocal)
    : false;
  const canUse = canUseAi && activePolicy !== "off" && policyAllows;

  const focusLabel = useFocusLabel(engagementId, focus);
  const quickPrompts = promptsFor(focus);

  const setPolicy = useMutation({
    mutationFn: (next: AiPolicy) => api.setAiPolicy(engagementId, next),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai-policy", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["audit", engagementId] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const startPreview = (message: string) => {
    setError(null);
    setPreviewMessage(message);
    previewMutation.mutate(message);
  };

  const previewMutation = useMutation({
    mutationFn: (message: string) => {
      const history: AssistantTurn[] = messages.slice(-10).map((item) => ({
        role: item.role,
        content: item.content,
      }));
      return api.previewAssistantChat(
        engagementId,
        selectedProviderId,
        message,
        history,
        focus,
        Array.from(fileIds),
      );
    },
    onSuccess: (result) => {
      setError(null);
      setPreview(result);
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const resolveTool = useMutation({
    mutationFn: ({
      request,
      decision,
    }: {
      request: AssistantToolRequest;
      decision: "allow_once" | "always" | "deny";
    }) => api.resolveAssistantTool(request.runId, request.callId, decision),
    onSuccess: (step) => {
      setToolRequest(null);
      handleStep(step);
    },
    onError: (mutationError) => {
      setToolRequest(null);
      setError(mutationError.message);
    },
  });

  const handleStep = (step: AssistantStep) => {
    if (step.kind === "final") {
      setMessages((current) => [
        ...current,
        { role: "user", content: previewMessage },
        {
          role: "assistant",
          content: step.reply.message,
          note: [step.reply.providerName, step.reply.model].join(" · "),
          proposal: step.reply.proposal ?? undefined,
        },
      ]);
      setPreview(null);
      setPrompt("");
      void queryClient.invalidateQueries({ queryKey: ["audit", engagementId] });
      return;
    }
    if (step.kind === "tool_request") {
      setPreview(null);
      setToolRequest(step);
      return;
    }
    setToolRequest(null);
    setPreview(null);
    setError(step.message);
  };

  const applyProposal = useMutation({
    mutationFn: ({ proposal }: { messageIndex: number; proposal: AgentProposal }) =>
      api.applyAgentProposal(proposal.proposalId),
    onSuccess: (result, variables) => {
      setMessages((current) =>
        current.map((message, index) =>
          index === variables.messageIndex
            ? { ...message, applied: true, note: result.message }
            : message,
        ),
      );
      void queryClient.invalidateQueries({ queryKey: ["engagement", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["engagements"] });
      void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
      void queryClient.invalidateQueries({ queryKey: ["test-cases", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["findings", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["audit", engagementId] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Sparkles className="h-4 w-4 text-emerald-400" /> Engagement assistant
          </h2>
          <p className="text-xs text-slate-500">
            I can read this engagement, draft tests and findings, summarize evidence, and propose
            changes. Every write needs your approval.
          </p>
        </div>
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="sm" onClick={() => setPermissionsOpen(true)}>
            <Settings2 className="h-3.5 w-3.5" /> Tool permissions
          </Button>
          {messages.length > 0 && (
            <Button variant="ghost" size="sm" onClick={() => setMessages(() => [])}>
              <Trash2 className="h-3.5 w-3.5" /> Clear
            </Button>
          )}
        </div>
      </div>

      {error && <ErrorBlock message={error} />}
      {providers.isLoading && <LoadingBlock label="Loading AI providers…" />}
      {providers.isError && <ErrorBlock message={providers.error.message} />}
      {policy.isError && <ErrorBlock message={policy.error.message} />}

      {providers.data && providers.data.length === 0 && (
        <div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-slate-800 px-6 py-14 text-center">
          <Sparkles className="h-6 w-6 text-slate-600" />
          <p className="text-sm font-medium text-slate-200">No AI provider configured</p>
          <p className="max-w-md text-xs text-slate-500">
            Add a local Ollama endpoint or an approved cloud provider in Settings, then return here.
          </p>
          <Link
            to="/settings"
            className="mt-2 inline-flex h-9 items-center rounded-lg bg-emerald-500 px-4 text-xs font-medium text-slate-950 hover:bg-emerald-400"
          >
            Open settings
          </Link>
        </div>
      )}

      {providers.data && providers.data.length > 0 && (
        <>
          <Card>
            <CardBody className="flex flex-wrap items-end gap-3">
              <Field label="Provider" className="min-w-56 flex-1">
                <Select value={selectedProviderId} onValueChange={setProviderId}>
                  {providers.data.map((provider) => (
                    <SelectItem key={provider.id} value={provider.id}>
                      {provider.name} · {provider.model}
                      {provider.isLocal ? " · local" : " · cloud"}
                      {provider.supportsVision ? " · vision" : ""}
                    </SelectItem>
                  ))}
                </Select>
              </Field>
              <Field label="AI policy" className="w-52">
                <Select
                  value={activePolicy}
                  disabled={!canManage || setPolicy.isPending}
                  onValueChange={(value) => setPolicy.mutate(value as AiPolicy)}
                >
                  {(Object.keys(policyLabel) as AiPolicy[]).map((value) => (
                    <SelectItem key={value} value={value}>
                      {policyLabel[value]}
                    </SelectItem>
                  ))}
                </Select>
              </Field>
              <div className="mb-0.5 flex flex-wrap items-center gap-2">
                <Badge tone={selectedProvider?.isLocal ? "success" : "warning"}>
                  {selectedProvider?.isLocal ? "Local" : "Cloud"}
                </Badge>
                {selectedProvider?.supportsVision && <Badge tone="accent">Vision</Badge>}
                <Badge tone="info">{policyLabel[activePolicy]}</Badge>
              </div>
            </CardBody>
          </Card>

          <Card>
            <CardHeader
              title="Reference files"
              description="Upload Postman, Burp XML, HAR, Procmon CSV/XML, OpenAPI, GraphQL, Nmap, or text files for this engagement."
              action={
                <label className="inline-flex cursor-pointer items-center gap-2 rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-xs text-slate-200 hover:border-slate-600">
                  <Upload className="h-3.5 w-3.5" />
                  Upload
                  <input
                    type="file"
                    className="hidden"
                    accept=".json,.xml,.har,.csv,.yaml,.yml,.graphql,.gql,.http,.txt,.md,.log"
                    onChange={(event) => {
                      const file = event.target.files?.[0];
                      if (file) uploadFile.mutate(file);
                      event.target.value = "";
                    }}
                  />
                </label>
              }
            />
            <CardBody className="space-y-3">
              {files.isLoading && <p className="text-xs text-slate-500">Loading files…</p>}
              {files.isError && <ErrorBlock message={files.error.message} />}
              {uploadFile.isPending && (
                <p className="text-xs text-slate-500">Parsing and encrypting…</p>
              )}
              {files.data && files.data.length === 0 && (
                <p className="text-xs text-slate-500">
                  No reference files yet. Upload the exports the assistant should review.
                </p>
              )}
              {files.data && files.data.length > 0 && (
                <div className="space-y-2">
                  {files.data.map((file) => (
                    <ReferenceFileRow
                      key={file.id}
                      file={file}
                      selected={fileIds.has(file.id)}
                      onToggle={() =>
                        setFileIds((current) => {
                          const next = new Set(current);
                          if (next.has(file.id)) next.delete(file.id);
                          else next.add(file.id);
                          return next;
                        })
                      }
                      onDelete={() => {
                        if (window.confirm(`Delete reference file "${file.filename}"?`)) {
                          deleteFile.mutate(file.id);
                        }
                      }}
                    />
                  ))}
                </div>
              )}
              <p className="text-[11px] text-slate-600">
                Selected file summaries are included in the assistant context. Detailed file content
                is read only through approved tool calls.
              </p>
            </CardBody>
          </Card>

          <Card>
            <CardHeader
              title="Conversation"
              description="Only this unlocked session keeps the conversation. Applying a proposal is explicit and audited."
              action={
                focus.kind !== "engagement" ? (
                  <button
                    type="button"
                    onClick={() => onFocusChange({ kind: "engagement", id: null })}
                    className="inline-flex items-center gap-1 rounded-full bg-slate-800 px-2 py-1 text-[11px] text-slate-300 hover:bg-slate-700"
                  >
                    <Target className="h-3 w-3" /> {focusLabel || focus.kind.replace("_", " ")}
                    <X className="h-3 w-3" />
                  </button>
                ) : undefined
              }
            />
            <CardBody className="space-y-4">
              <div className="max-h-[52vh] space-y-4 overflow-y-auto pr-1">
                {messages.length === 0 ? (
                  <div className="flex flex-col items-center justify-center gap-2 py-10 text-center">
                    <Bot className="h-7 w-7 text-emerald-400/70" />
                    <p className="text-sm text-slate-300">What are we working on?</p>
                    <p className="max-w-lg text-xs text-slate-500">
                      Ask about the engagement, draft tests, review findings, or analyze evidence.
                      The assistant decides which action fits and shows it for approval.
                    </p>
                    <div className="mt-2 flex flex-wrap justify-center gap-2">
                      {quickPrompts.map((item) => (
                        <Button
                          key={item}
                          size="sm"
                          variant="secondary"
                          onClick={() => startPreview(item)}
                          disabled={!canUse || previewMutation.isPending}
                        >
                          {item}
                        </Button>
                      ))}
                    </div>
                  </div>
                ) : (
                  messages.map((message, index) => (
                    <div
                      key={`${message.role}-${index}`}
                      className={
                        message.role === "user"
                          ? "ml-8 rounded-lg border border-slate-700 bg-slate-800/40 p-3"
                          : "mr-8 rounded-lg border border-emerald-500/15 bg-emerald-500/5 p-3"
                      }
                    >
                      <div className="mb-1 flex items-center gap-2 text-[11px] font-medium text-slate-400">
                        {message.role === "user" ? (
                          <Lock className="h-3 w-3" />
                        ) : (
                          <Bot className="h-3 w-3" />
                        )}
                        {message.role === "user" ? "You" : "Assistant"}
                        {message.note && (
                          <span className="font-normal text-slate-600">{message.note}</span>
                        )}
                      </div>
                      <p className="whitespace-pre-wrap text-xs leading-relaxed text-slate-200">
                        {message.content}
                      </p>
                      {message.proposal && (
                        <ProposalCard
                          proposal={message.proposal}
                          applied={Boolean(message.applied)}
                          busy={applyProposal.isPending}
                          onApply={() =>
                            applyProposal.mutate({
                              messageIndex: index,
                              proposal: message.proposal!,
                            })
                          }
                          onDiscard={() =>
                            setMessages((current) =>
                              current.map((item, itemIndex) =>
                                itemIndex === index ? { ...item, proposal: undefined } : item,
                              ),
                            )
                          }
                        />
                      )}
                    </div>
                  ))
                )}
              </div>

              {activePolicy === "off" && (
                <PolicyWarning>AI is disabled for this engagement.</PolicyWarning>
              )}
              {activePolicy === "local_only" && selectedProvider && !selectedProvider.isLocal && (
                <PolicyWarning>
                  This provider is not local. Set the engagement policy to approved cloud to use it.
                </PolicyWarning>
              )}

              <div className="flex items-end gap-2 border-t border-slate-800 pt-4">
                <Textarea
                  rows={3}
                  value={prompt}
                  onChange={(event) => setPrompt(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" && !event.shiftKey && prompt.trim() && canUse) {
                      event.preventDefault();
                      startPreview(prompt.trim());
                    }
                  }}
                  placeholder={
                    focus.kind === "evidence"
                      ? "Ask about this evidence, for example: summarize it and identify security-relevant details."
                      : "Ask the assistant. It selects the action and asks for approval before writing."
                  }
                  className="min-h-20 flex-1"
                />
                <Button
                  onClick={() => startPreview(prompt.trim())}
                  disabled={!canUse || !prompt.trim() || previewMutation.isPending}
                >
                  {previewMutation.isPending ? <Spinner /> : <Send className="h-4 w-4" />}
                  Preview
                </Button>
              </div>
              <p className="text-[11px] text-slate-600">
                The exact redacted prompt is shown before anything leaves the device. Images and
                evidence text require an explicit action. Redaction does not cover image pixels.
              </p>
            </CardBody>
          </Card>
        </>
      )}

      {preview && (
        <AssistantPreviewDialog
          preview={preview}
          message={previewMessage}
          onClose={() => setPreview(null)}
          onStep={handleStep}
        />
      )}
      {toolRequest && (
        <ToolRequestDialog
          request={toolRequest}
          busy={resolveTool.isPending}
          onDecision={(decision) => resolveTool.mutate({ request: toolRequest, decision })}
        />
      )}
      {permissionsOpen && (
        <ToolPermissionsDialog
          permissions={permissions.data ?? []}
          loading={permissions.isLoading}
          onClose={() => setPermissionsOpen(false)}
          onRevoke={(tool) => revokePermission.mutate(tool)}
        />
      )}
    </div>
  );
}

function ProposalCard({
  proposal,
  applied,
  busy,
  onApply,
  onDiscard,
}: {
  proposal: AgentProposal;
  applied: boolean;
  busy: boolean;
  onApply: () => void;
  onDiscard: () => void;
}) {
  return (
    <div className="mt-3 rounded-lg border border-emerald-500/25 bg-slate-950/40 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <Badge tone="accent">{skillLabel[proposal.skill]}</Badge>
        {proposal.targetId && <Badge tone="info">{proposal.targetId.slice(0, 8)}</Badge>}
        {applied && <Badge tone="success">Applied</Badge>}
      </div>
      <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-900/70 p-2 text-[11px] leading-relaxed text-slate-300">
        {JSON.stringify(proposal.payload, null, 2)}
      </pre>
      {!applied && (
        <div className="mt-2 flex justify-end gap-2">
          <Button size="sm" variant="secondary" onClick={onDiscard} disabled={busy}>
            Discard
          </Button>
          <Button size="sm" onClick={onApply} disabled={busy}>
            {busy ? <Spinner /> : <CheckCircle2 className="h-3.5 w-3.5" />} Apply
          </Button>
        </div>
      )}
    </div>
  );
}

function AssistantPreviewDialog({
  preview,
  message,
  onClose,
  onStep,
}: {
  preview: AssistantPreview;
  message: string;
  onClose: () => void;
  onStep: (step: AssistantStep) => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const send = useMutation({
    mutationFn: () => api.runAssistantChat(preview.consentId),
    onSuccess: (step) => {
      onStep(step);
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  return (
    <Dialog
      open
      onOpenChange={(next) => !next && onClose()}
      title="Review assistant request"
      description="Only the redacted prompt below is sent. Consent expires in five minutes and can be used once."
      wide
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={() => send.mutate()} disabled={send.isPending}>
            {send.isPending ? <Spinner /> : <Send className="h-4 w-4" />}
            Send request
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div className="flex flex-wrap items-center gap-2">
          <Badge tone="accent">{preview.providerName}</Badge>
          <Badge>{preview.model}</Badge>
          <Badge tone={preview.isLocal ? "success" : "warning"}>
            {preview.isLocal ? "Local" : "Cloud"}
          </Badge>
          <Badge tone="info">{policyLabel[preview.policy]}</Badge>
          <Badge tone="neutral">{preview.focus.kind.replace("_", " ")}</Badge>
          {preview.targetLabel && <Badge tone="accent">{preview.targetLabel}</Badge>}
          {preview.redactionCount > 0 && (
            <Badge tone="danger">{preview.redactionCount} redaction(s)</Badge>
          )}
        </div>
        {preview.attachment && (
          <div className="flex flex-wrap items-center gap-3 rounded-lg border border-violet-500/25 bg-violet-500/5 p-3 text-[11px] text-violet-200">
            {preview.attachment.mimeType.startsWith("image/") ? (
              <Image className="h-4 w-4" />
            ) : (
              <FileText className="h-4 w-4" />
            )}
            <span>
              {preview.attachment.filename} · {preview.attachment.mimeType} ·{" "}
              {Math.ceil(preview.attachment.size / 1024)} KB · {preview.attachment.width}×
              {preview.attachment.height}
            </span>
            <span className="font-mono text-violet-300/60">
              sent hash {preview.attachment.hash.slice(0, 12)}…
            </span>
          </div>
        )}
        <div className="rounded-lg border border-slate-800 bg-slate-950/40 px-3 py-2 text-[11px] text-slate-400">
          Your message: <span className="text-slate-200">{message}</span>
        </div>
        <pre className="max-h-[50vh] overflow-auto whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs leading-relaxed text-slate-200">
          {preview.redactedPrompt}
        </pre>
        <div className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[11px] text-amber-200/80">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          Redaction is best-effort and does not cover text inside images. Review the preview before
          sending.
        </div>
        {error && <ErrorBlock message={error} />}
      </div>
    </Dialog>
  );
}

function useFocusLabel(engagementId: string, focus: AssistantFocus): string | null {
  const findings = useQuery({
    queryKey: ["findings", engagementId],
    queryFn: () => api.listFindings(engagementId),
    enabled: focus.kind === "finding" || focus.kind === "test_case",
  });
  const evidence = useQuery({
    queryKey: ["evidence", engagementId],
    queryFn: () => api.listEvidence(engagementId),
    enabled: focus.kind === "evidence",
  });
  const tests = useQuery({
    queryKey: ["test-cases", engagementId],
    queryFn: () => api.listTestCases(engagementId),
    enabled: focus.kind === "test_case" || focus.kind === "finding",
  });
  if (focus.kind === "finding") {
    const finding = findings.data?.find((item) => item.id === focus.id);
    return finding ? `${finding.code} · ${finding.title}` : null;
  }
  if (focus.kind === "test_case") {
    const test = tests.data?.find((item) => item.id === focus.id);
    return test ? `${test.title}${test.category ? ` · ${test.category}` : ""}` : null;
  }
  if (focus.kind === "evidence") {
    return evidence.data?.find((item) => item.id === focus.id)?.filename ?? null;
  }
  return "Engagement";
}

function promptsFor(focus: AssistantFocus): string[] {
  if (focus.kind === "finding") {
    return [
      "Review this finding and improve the description, impact, and remediation.",
      "Is the severity justified by the recorded evidence?",
      "What evidence is missing?",
    ];
  }
  if (focus.kind === "evidence") {
    return [
      "Summarize this evidence and identify security-relevant details.",
      "Does this evidence support a finding?",
      "What follow-up testing should I do?",
    ];
  }
  if (focus.kind === "test_case") {
    return [
      "Draft a detailed test approach for this test case.",
      "Does the recorded result support a finding?",
      "What should I test next?",
    ];
  }
  return [
    "What should I test next for this engagement?",
    "Review the open findings and identify gaps.",
    "Draft missing authorization and authentication tests.",
    "Draft an executive summary narrative.",
  ];
}

function ReferenceFileRow({
  file,
  selected,
  onToggle,
  onDelete,
}: {
  file: EngagementFile;
  selected: boolean;
  onToggle: () => void;
  onDelete: () => void;
}) {
  const summary = file.summary as Record<string, unknown>;
  const count =
    (summary.requests as number | undefined) ??
    (summary.rows as number | undefined) ??
    (summary.hosts as number | undefined) ??
    (summary.operations as number | undefined) ??
    (summary.items as number | undefined);
  return (
    <div className="flex items-start gap-3 rounded-lg border border-slate-800 bg-slate-950/40 p-3">
      <input
        type="checkbox"
        checked={selected}
        onChange={onToggle}
        className="mt-0.5 h-4 w-4 rounded border-slate-600 bg-slate-900"
      />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-1.5">
          <p className="truncate text-xs font-medium text-slate-200">{file.filename}</p>
          <Badge tone="accent">{file.kind}</Badge>
          <span className="text-[11px] text-slate-500">{formatBytes(file.size)}</span>
        </div>
        <p className="mt-1 text-[11px] text-slate-500">
          {count != null ? `${count} item${count === 1 ? "" : "s"}` : "Parsed reference"}
        </p>
      </div>
      <Button variant="ghost" size="icon" aria-label="Delete reference file" onClick={onDelete}>
        <Trash2 className="h-3.5 w-3.5 text-slate-500" />
      </Button>
    </div>
  );
}

function ToolRequestDialog({
  request,
  busy,
  onDecision,
}: {
  request: AssistantToolRequest;
  busy: boolean;
  onDecision: (decision: "allow_once" | "always" | "deny") => void;
}) {
  return (
    <Dialog
      open
      onOpenChange={(next) => {
        if (!next) onDecision("deny");
      }}
      title="Approve assistant tool call"
      description="The assistant wants to read selected local data. Nothing is written by a tool call."
      footer={
        <>
          <Button variant="secondary" onClick={() => onDecision("deny")} disabled={busy}>
            Deny and stop
          </Button>
          <Button variant="secondary" onClick={() => onDecision("always")} disabled={busy}>
            Always allow
          </Button>
          <Button onClick={() => onDecision("allow_once")} disabled={busy}>
            {busy ? <Spinner /> : <CheckCircle2 className="h-4 w-4" />} Allow once
          </Button>
        </>
      }
    >
      <div className="space-y-3">
        <div className="flex flex-wrap items-center gap-2">
          <Badge tone="accent">{request.tool}</Badge>
          <span className="text-[11px] text-slate-500">run {request.runId.slice(0, 8)}…</span>
        </div>
        <p className="text-xs text-slate-400">
          Allow always applies only to this tool in this engagement. You can remove it from Tool
          permissions at any time.
        </p>
        <pre className="max-h-64 overflow-auto whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs text-slate-300">
          {JSON.stringify(request.arguments, null, 2)}
        </pre>
      </div>
    </Dialog>
  );
}

function ToolPermissionsDialog({
  permissions,
  loading,
  onClose,
  onRevoke,
}: {
  permissions: ToolPermission[];
  loading: boolean;
  onClose: () => void;
  onRevoke: (tool: string) => void;
}) {
  return (
    <Dialog
      open
      onOpenChange={(next) => !next && onClose()}
      title="Tool permissions"
      description="These tools no longer prompt in this engagement."
      footer={
        <Button variant="secondary" onClick={onClose}>
          Close
        </Button>
      }
    >
      {loading && <p className="text-xs text-slate-500">Loading permissions…</p>}
      {!loading && permissions.length === 0 && (
        <p className="text-xs text-slate-500">No always-allowed tools.</p>
      )}
      {permissions.length > 0 && (
        <div className="space-y-2">
          {permissions.map((permission) => (
            <div
              key={permission.tool}
              className="flex items-center justify-between gap-3 rounded-lg border border-slate-800 bg-slate-950/40 px-3 py-2"
            >
              <span className="font-mono text-xs text-slate-200">{permission.tool}</span>
              <Button size="sm" variant="secondary" onClick={() => onRevoke(permission.tool)}>
                Remove
              </Button>
            </div>
          ))}
        </div>
      )}
    </Dialog>
  );
}

function PolicyWarning({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[11px] text-amber-200/80">
      <ShieldCheck className="mt-0.5 h-3.5 w-3.5 shrink-0" />
      <span>{children}</span>
    </div>
  );
}
