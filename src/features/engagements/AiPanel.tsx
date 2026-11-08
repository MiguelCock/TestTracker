import { useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, Bot, Lock, Send, ShieldCheck, Sparkles, Trash2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { Field, Select, Textarea } from "@/components/ui/input";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { useSession } from "@/hooks/useSession";
import { api } from "@/lib/api";
import type { AiPolicy, AiPromptPreview, AiPurpose } from "@/types";

const policyLabel: Record<AiPolicy, string> = {
  off: "Off",
  local_only: "Local models only",
  approved_cloud: "Approved cloud providers",
};

const purposeLabel: Record<AiPurpose, string> = {
  general: "General assistant",
  test_planning: "Test planning",
  finding_draft: "Finding draft",
  evidence_summary: "Evidence summary",
  report_draft: "Report drafting",
  review: "Report review",
};

type ChatMessage =
  | { role: "user"; content: string; note: string }
  | { role: "assistant"; content: string; note: string };

export function AiPanel({ engagementId }: { engagementId: string }) {
  const queryClient = useQueryClient();
  const session = useSession();
  const providers = useQuery({ queryKey: ["ai-providers"], queryFn: api.listAiProviders });
  const policy = useQuery({
    queryKey: ["ai-policy", engagementId],
    queryFn: () => api.getAiPolicy(engagementId),
  });
  const [providerId, setProviderId] = useState("");
  const [purpose, setPurpose] = useState<AiPurpose>("general");
  const [prompt, setPrompt] = useState("");
  const [preview, setPreview] = useState<AiPromptPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const conversation = useQuery<ChatMessage[]>({
    queryKey: ["ai-conversation", engagementId],
    queryFn: () => [],
    staleTime: Infinity,
    gcTime: Infinity,
  });
  const messages = conversation.data ?? [];
  const setMessages = (updater: (current: ChatMessage[]) => ChatMessage[]) => {
    queryClient.setQueryData<ChatMessage[]>(["ai-conversation", engagementId], (current) =>
      updater(current ?? []),
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

  const setPolicy = useMutation({
    mutationFn: (next: AiPolicy) => api.setAiPolicy(engagementId, next),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai-policy", engagementId] });
      void queryClient.invalidateQueries({ queryKey: ["audit", engagementId] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const previewMutation = useMutation({
    mutationFn: () =>
      api.previewAiPrompt({
        engagementId,
        providerId: selectedProviderId,
        purpose,
        prompt,
      }),
    onSuccess: (result) => {
      setError(null);
      setPreview(result);
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-white">AI assistant</h2>
          <p className="text-xs text-slate-500">
            Prompts are redacted, previewed, and consented before leaving the device.
          </p>
        </div>
        {messages.length > 0 && (
          <Button variant="ghost" size="sm" onClick={() => setMessages(() => [])}>
            <Trash2 className="h-3.5 w-3.5" /> Clear conversation
          </Button>
        )}
      </div>

      {error && <ErrorBlock message={error} />}
      {providers.isLoading && <LoadingBlock label="Loading AI providers…" />}
      {providers.isError && <ErrorBlock message={providers.error.message} />}
      {policy.isError && <ErrorBlock message={policy.error.message} />}

      {providers.data && providers.data.length === 0 && (
        <EmptyState
          title="No AI providers configured"
          description="Add a local Ollama endpoint or a cloud provider in Settings. Cloud use stays disabled per engagement until approved."
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

      {providers.data && providers.data.length > 0 && (
        <div className="grid gap-4 xl:grid-cols-[380px_1fr]">
          <Card>
            <CardHeader title="Prompt" description="Only the redacted preview is sent." />
            <CardBody className="space-y-4">
              <Field
                label="Engagement AI policy"
                hint="Local-only is the default. Cloud requires explicit approval."
              >
                <Select
                  value={activePolicy}
                  disabled={!canManage || setPolicy.isPending}
                  onChange={(event) => setPolicy.mutate(event.target.value as AiPolicy)}
                >
                  {(Object.keys(policyLabel) as AiPolicy[]).map((value) => (
                    <option key={value} value={value}>
                      {policyLabel[value]}
                    </option>
                  ))}
                </Select>
              </Field>
              <Field label="Provider">
                <Select
                  value={selectedProviderId}
                  onChange={(event) => setProviderId(event.target.value)}
                >
                  {providers.data.map((provider) => (
                    <option key={provider.id} value={provider.id}>
                      {provider.name} · {provider.model}
                      {provider.isLocal ? " · local" : " · cloud"}
                    </option>
                  ))}
                </Select>
              </Field>
              <Field label="Purpose">
                <Select
                  value={purpose}
                  onChange={(event) => setPurpose(event.target.value as AiPurpose)}
                >
                  {(Object.keys(purposeLabel) as AiPurpose[]).map((value) => (
                    <option key={value} value={value}>
                      {purposeLabel[value]}
                    </option>
                  ))}
                </Select>
              </Field>
              <Field label="Prompt">
                <Textarea
                  rows={7}
                  value={prompt}
                  onChange={(event) => setPrompt(event.target.value)}
                  placeholder="Paste only the context you intend to share. Do not include credentials or unnecessary client data."
                />
              </Field>

              {activePolicy === "off" && (
                <PolicyWarning>
                  AI is disabled for this engagement. Switch the policy to local-only or approved
                  cloud to use the assistant.
                </PolicyWarning>
              )}
              {activePolicy === "local_only" && selectedProvider && !selectedProvider.isLocal && (
                <PolicyWarning>
                  This provider is not local. Set the engagement policy to approved cloud to use it.
                </PolicyWarning>
              )}
              {selectedProvider && (
                <div className="flex flex-wrap items-center gap-2 text-[11px] text-slate-500">
                  <Badge tone={selectedProvider.isLocal ? "success" : "warning"}>
                    {selectedProvider.isLocal ? "Local" : "Cloud"}
                  </Badge>
                  <span>{selectedProvider.model}</span>
                  <span>·</span>
                  <span>{policyLabel[activePolicy]}</span>
                </div>
              )}

              <Button
                className="w-full"
                onClick={() => previewMutation.mutate()}
                disabled={!canUse || !prompt.trim() || previewMutation.isPending}
              >
                {previewMutation.isPending ? <Spinner /> : <ShieldCheck className="h-4 w-4" />}
                Preview redactions
              </Button>
            </CardBody>
          </Card>

          <Card>
            <CardHeader
              title="Conversation"
              description="Responses are drafts. They stay only for this unlocked session and are never saved to the engagement."
            />
            <CardBody>
              {messages.length === 0 ? (
                <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
                  <Sparkles className="h-6 w-6 text-slate-600" />
                  <p className="text-sm text-slate-300">No messages yet</p>
                  <p className="max-w-sm text-xs text-slate-500">
                    Preview a prompt to see exactly what will be sent. Sending requires a fresh,
                    one-time consent.
                  </p>
                </div>
              ) : (
                <div className="space-y-4">
                  {messages.map((message, index) => (
                    <div
                      key={index}
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
                        {message.role === "user" ? "Redacted prompt" : "Assistant draft"}
                        <span className="font-normal text-slate-600">{message.note}</span>
                      </div>
                      <p className="whitespace-pre-wrap text-xs leading-relaxed text-slate-200">
                        {message.content}
                      </p>
                    </div>
                  ))}
                </div>
              )}
            </CardBody>
          </Card>
        </div>
      )}

      {preview && (
        <PreviewDialog
          preview={preview}
          onClose={() => setPreview(null)}
          onSent={(userMessage, assistantMessage) => {
            setMessages((current) => [...current, userMessage, assistantMessage]);
            setPreview(null);
            setPrompt("");
            void queryClient.invalidateQueries({ queryKey: ["audit", engagementId] });
          }}
        />
      )}
    </div>
  );
}

function PreviewDialog({
  preview,
  onClose,
  onSent,
}: {
  preview: AiPromptPreview;
  onClose: () => void;
  onSent: (userMessage: ChatMessage, assistantMessage: ChatMessage) => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const send = useMutation({
    mutationFn: () => api.sendAiMessage(preview.consentId),
    onSuccess: (response) => {
      onSent(
        {
          role: "user",
          content: preview.redactedPrompt,
          note: preview.redactionCount > 0 ? `${preview.redactionCount} redaction(s)` : "",
        },
        {
          role: "assistant",
          content: response.content,
          note: [
            response.providerName,
            response.model,
            response.promptTokens != null
              ? `${response.promptTokens} prompt / ${response.completionTokens ?? 0} completion tokens`
              : null,
          ]
            .filter(Boolean)
            .join(" · "),
        },
      );
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  return (
    <Dialog
      open
      onOpenChange={(next) => !next && onClose()}
      title="Review AI payload"
      description="Only the redacted prompt below is sent. Consent expires in five minutes and can be used once."
      wide
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={() => send.mutate()} disabled={send.isPending}>
            {send.isPending ? <Spinner /> : <Send className="h-4 w-4" />}
            Send redacted prompt
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
          <Badge tone={preview.policy === "approved_cloud" ? "warning" : "info"}>
            {policyLabel[preview.policy]}
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

        <div className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[11px] text-amber-200/80">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          Redaction is best-effort. Review the preview before sending. This consent cannot be
          reused.
        </div>
        {error && <ErrorBlock message={error} />}
        <p className="text-[11px] text-slate-600">
          The audit log records the provider, model, purpose, prompt hash, response hash, and
          redaction count. Message content is not persisted.
        </p>
      </div>
    </Dialog>
  );
}

function PolicyWarning({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[11px] text-amber-200/80">
      <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
      {children}
    </div>
  );
}
