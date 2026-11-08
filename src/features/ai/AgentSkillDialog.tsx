import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Bot, ShieldCheck, Sparkles } from "lucide-react";
import { Badge, SeverityBadge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Field, Textarea } from "@/components/ui/input";
import { Select, SelectItem } from "@/components/ui/select";
import { ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import { severityFor } from "@/lib/severity";
import type { AgentApplyResult, AgentPreview, AgentProposal, AgentSkill, AiPolicy } from "@/types";

const skillLabel: Record<AgentSkill, string> = {
  application_summary: "Application summary",
  attack_surface: "Attack surface",
  assumptions: "Assumptions",
  test_cases: "Test cases",
  test_approach: "Test approach",
  finding_draft: "Finding draft",
};

const policyLabel: Record<AiPolicy, string> = {
  off: "Off",
  local_only: "Local models only",
  approved_cloud: "Approved cloud providers",
};

export function AgentSkillDialog({
  open,
  onOpenChange,
  engagementId,
  skills,
  targetId,
  targetLabel,
  onApplied,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  engagementId: string;
  skills: AgentSkill[];
  targetId?: string;
  targetLabel?: string;
  onApplied: (result: AgentApplyResult) => void;
}) {
  const [skill, setSkill] = useState<AgentSkill>(skills[0]);
  const [providerId, setProviderId] = useState("");
  const [instructions, setInstructions] = useState("");
  const [preview, setPreview] = useState<AgentPreview | null>(null);
  const [proposal, setProposal] = useState<AgentProposal | null>(null);
  const [error, setError] = useState<string | null>(null);

  const providers = useQuery({ queryKey: ["ai-providers"], queryFn: api.listAiProviders });
  const policy = useQuery({
    queryKey: ["ai-policy", engagementId],
    queryFn: () => api.getAiPolicy(engagementId),
  });
  const selectedProviderId = providerId || providers.data?.[0]?.id || "";
  const selectedProvider = providers.data?.find((provider) => provider.id === selectedProviderId);
  const activePolicy = policy.data ?? "local_only";
  const canUse =
    Boolean(selectedProvider) &&
    (activePolicy === "approved_cloud" ||
      (activePolicy === "local_only" && Boolean(selectedProvider?.isLocal)));

  const previewMutation = useMutation({
    mutationFn: () =>
      api.previewAgentSkill(
        engagementId,
        selectedProviderId,
        skill,
        targetId ?? null,
        instructions,
      ),
    onSuccess: (result) => {
      setError(null);
      setPreview(result);
      setProposal(null);
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const runMutation = useMutation({
    mutationFn: () => api.runAgentSkill(preview!.consentId),
    onSuccess: (result) => {
      setError(null);
      setProposal(result);
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const applyMutation = useMutation({
    mutationFn: () => api.applyAgentProposal(proposal!.proposalId),
    onSuccess: (result) => {
      onApplied(result);
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
      title={proposal ? "Review agent proposal" : "Agent skill"}
      description={
        proposal
          ? "Review the proposal and apply it explicitly. Nothing is written automatically."
          : "The server builds a scoped context, redacts it, and shows exactly what will be sent."
      }
      wide
      footer={
        proposal ? (
          <>
            <Button variant="secondary" onClick={() => onOpenChange(false)}>
              Discard
            </Button>
            <Button onClick={() => applyMutation.mutate()} disabled={applyMutation.isPending}>
              {applyMutation.isPending ? <Spinner /> : <Sparkles className="h-4 w-4" />}
              Apply proposal
            </Button>
          </>
        ) : preview ? (
          <>
            <Button variant="secondary" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button onClick={() => runMutation.mutate()} disabled={runMutation.isPending}>
              {runMutation.isPending ? <Spinner /> : <Bot className="h-4 w-4" />}
              Run agent
            </Button>
          </>
        ) : (
          <>
            <Button variant="secondary" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => previewMutation.mutate()}
              disabled={!canUse || previewMutation.isPending}
            >
              {previewMutation.isPending ? <Spinner /> : <ShieldCheck className="h-4 w-4" />}
              Preview context
            </Button>
          </>
        )
      }
    >
      {proposal ? (
        <ProposalView proposal={proposal} />
      ) : preview ? (
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
          {preview.targetLabel && (
            <p className="text-xs text-slate-400">Target: {preview.targetLabel}</p>
          )}
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
            This preview is the consent. Running the agent may make one provider call. The proposal
            still requires explicit approval.
          </p>
          {error && <ErrorBlock message={error} />}
        </div>
      ) : (
        <div className="space-y-4">
          {providers.isLoading && <LoadingBlock label="Loading providers…" />}
          {providers.isError && <ErrorBlock message={providers.error.message} />}
          {providers.data && providers.data.length === 0 && (
            <ErrorBlock message="Configure an AI provider in Settings before using agent skills." />
          )}
          {providers.data && providers.data.length > 0 && (
            <>
              {skills.length > 1 && (
                <Field label="Skill">
                  <Select
                    value={skill}
                    onValueChange={(value) => {
                      setSkill(value as AgentSkill);
                      setPreview(null);
                    }}
                  >
                    {skills.map((value) => (
                      <SelectItem key={value} value={value}>
                        {skillLabel[value]}
                      </SelectItem>
                    ))}
                  </Select>
                </Field>
              )}
              <Field label="Provider">
                <Select value={selectedProviderId} onValueChange={setProviderId}>
                  {providers.data.map((provider) => (
                    <SelectItem key={provider.id} value={provider.id}>
                      {provider.name} · {provider.model}
                      {provider.isLocal ? " · local" : " · cloud"}
                    </SelectItem>
                  ))}
                </Select>
              </Field>
              {targetLabel && (
                <p className="rounded-lg border border-slate-800 bg-slate-950/40 px-3 py-2 text-xs text-slate-400">
                  Target: {targetLabel}
                </p>
              )}
              <Field
                label="Additional instructions"
                hint="Optional; included in the redacted prompt."
              >
                <Textarea
                  rows={3}
                  value={instructions}
                  onChange={(event) => setInstructions(event.target.value)}
                  placeholder="Focus areas, tone, or constraints for this draft."
                />
              </Field>
              {activePolicy === "off" && (
                <ErrorBlock message="AI is disabled for this engagement." />
              )}
              {activePolicy === "local_only" && selectedProvider && !selectedProvider.isLocal && (
                <ErrorBlock message="This provider is not local. Set the engagement AI policy to approved cloud." />
              )}
              {selectedProvider && (
                <div className="flex flex-wrap items-center gap-2 text-[11px] text-slate-500">
                  <Badge tone={selectedProvider.isLocal ? "success" : "warning"}>
                    {selectedProvider.isLocal ? "Local" : "Cloud"}
                  </Badge>
                  <Badge tone="info">{policyLabel[activePolicy]}</Badge>
                  <span>{skillLabel[skill]}</span>
                </div>
              )}
            </>
          )}
          {error && <ErrorBlock message={error} />}
        </div>
      )}
    </Dialog>
  );
}

function ProposalView({ proposal }: { proposal: AgentProposal }) {
  const payload = proposal.payload;
  if (proposal.skill === "application_summary" || proposal.skill === "attack_surface") {
    return (
      <p className="whitespace-pre-wrap text-sm leading-relaxed text-slate-200">
        {asString(payload.value)}
      </p>
    );
  }
  if (proposal.skill === "assumptions") {
    const values = Array.isArray(payload.values) ? payload.values : [];
    return (
      <ul className="space-y-2">
        {values.map((value, index) => (
          <li key={index} className="flex gap-2 text-xs text-slate-300">
            <span className="mt-1 h-1 w-1 shrink-0 rounded-full bg-emerald-400" />
            {String(value)}
          </li>
        ))}
      </ul>
    );
  }
  if (proposal.skill === "test_cases") {
    const items = Array.isArray(payload.testCases) ? payload.testCases : [];
    return (
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
            {items.map((item, index) => {
              const record = item as Record<string, unknown>;
              return (
                <tr key={index}>
                  <td className="px-3 py-2 font-medium text-slate-200">{asString(record.title)}</td>
                  <td className="px-3 py-2 text-slate-400">{asString(record.category)}</td>
                  <td className="px-3 py-2 text-slate-400">{asString(record.approach)}</td>
                  <td className="px-3 py-2 text-slate-400">{asString(record.expectedResult)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    );
  }
  if (proposal.skill === "test_approach") {
    return (
      <pre className="max-h-[60vh] overflow-auto whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs leading-relaxed text-slate-200">
        {asString(payload.approach)}
      </pre>
    );
  }
  const probability = Number(payload.probability ?? 0);
  const impact = Number(payload.impactScore ?? 0);
  const severity = severityFor(
    Math.min(9, Math.max(0, probability)),
    Math.min(9, Math.max(0, impact)),
  );
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-2">
        <p className="text-sm font-medium text-white">{asString(payload.title)}</p>
        <SeverityBadge severity={severity} />
        <Badge>{asString(payload.owasp) || "No OWASP"}</Badge>
        <Badge>{asString(payload.cwe) || "No CWE"}</Badge>
      </div>
      <p className="whitespace-pre-wrap text-xs leading-relaxed text-slate-300">
        {asString(payload.description)}
      </p>
      <p className="text-xs text-slate-400">Impact: {asString(payload.impact)}</p>
      <p className="text-xs text-slate-400">Remediation: {asString(payload.remediation)}</p>
    </div>
  );
}

function asString(value: unknown): string {
  return typeof value === "string" ? value : "";
}
