import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bot, Cloud, HardDrive, KeyRound, Pencil, Plus, RefreshCw, Trash2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { Field, Input, Select } from "@/components/ui/input";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { useSession } from "@/hooks/useSession";
import { api } from "@/lib/api";
import type { AiProviderInput, AiProviderKind, AiProviderSummary } from "@/types";

const kindLabel: Record<AiProviderKind, string> = {
  ollama: "Ollama / local",
  openai_compatible: "OpenAI-compatible",
  anthropic: "Anthropic",
  azure_openai: "Azure OpenAI",
};

const kindHint: Record<AiProviderKind, string> = {
  ollama:
    "Local endpoint, for example http://localhost:11434. Pull a model first: ollama pull llama3.2.",
  openai_compatible: "Base URL including /v1, or the full /chat/completions endpoint.",
  anthropic: "https://api.anthropic.com and an Anthropic API key.",
  azure_openai: "Full Azure chat completions endpoint including the api-version query parameter.",
};

const kindPlaceholder: Record<AiProviderKind, string> = {
  ollama: "http://localhost:11434",
  openai_compatible: "https://api.example.com/v1",
  anthropic: "https://api.anthropic.com",
  azure_openai:
    "https://resource.openai.azure.com/openai/deployments/deployment/chat/completions?api-version=...",
};

export function AiProvidersCard() {
  const queryClient = useQueryClient();
  const session = useSession();
  const canManage =
    session.data?.profile?.role === "admin" || session.data?.profile?.role === "lead";
  const [providerOpen, setProviderOpen] = useState(false);
  const [editing, setEditing] = useState<AiProviderSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const providers = useQuery({ queryKey: ["ai-providers"], queryFn: api.listAiProviders });
  const remove = useMutation({
    mutationFn: (id: string) => api.deleteAiProvider(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["ai-providers"] });
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  return (
    <Card className="mt-4">
      <CardHeader
        title="AI providers"
        description="Provider credentials are encrypted with the vault key and never returned to the interface."
        action={
          canManage ? (
            <Button
              size="sm"
              onClick={() => {
                setEditing(null);
                setProviderOpen(true);
              }}
            >
              <Plus className="h-3.5 w-3.5" /> Add provider
            </Button>
          ) : undefined
        }
      />
      <CardBody className="p-0">
        {error && (
          <div className="p-5 pb-0">
            <ErrorBlock message={error} />
          </div>
        )}
        {providers.isLoading && <LoadingBlock label="Loading providers…" />}
        {providers.isError && (
          <div className="p-5">
            <ErrorBlock message={providers.error.message} />
          </div>
        )}
        {providers.data && providers.data.length === 0 && (
          <div className="p-5">
            <EmptyState
              title="No AI providers"
              description="Add a local Ollama endpoint or a cloud provider. Cloud use is disabled per engagement until approved."
              action={
                <Button
                  size="sm"
                  onClick={() => {
                    setEditing(null);
                    setProviderOpen(true);
                  }}
                >
                  <Plus className="h-3.5 w-3.5" /> Add provider
                </Button>
              }
            />
          </div>
        )}
        {providers.data && providers.data.length > 0 && (
          <ul className="divide-y divide-slate-800">
            {providers.data.map((provider) => (
              <li
                key={provider.id}
                className="flex flex-wrap items-center justify-between gap-3 px-5 py-3"
              >
                <div className="flex min-w-0 items-start gap-3">
                  <div className="mt-0.5 text-slate-500">
                    {provider.isLocal ? (
                      <HardDrive className="h-4 w-4" />
                    ) : (
                      <Cloud className="h-4 w-4" />
                    )}
                  </div>
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="text-xs font-medium text-slate-200">{provider.name}</p>
                      <Badge tone={provider.isLocal ? "success" : "warning"}>
                        {provider.isLocal ? "Local" : "Cloud"}
                      </Badge>
                      {provider.hasKey && (
                        <Badge>
                          <KeyRound className="mr-1 h-3 w-3" /> Key stored
                        </Badge>
                      )}
                    </div>
                    <p className="mt-0.5 text-[11px] text-slate-500">
                      {kindLabel[provider.kind]} · {provider.model} · {provider.baseUrl}
                    </p>
                  </div>
                </div>
                {canManage && (
                  <div className="flex gap-1">
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label="Edit AI provider"
                      onClick={() => {
                        setEditing(provider);
                        setProviderOpen(true);
                      }}
                    >
                      <Pencil className="h-3.5 w-3.5" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label="Delete AI provider"
                      onClick={() => {
                        if (window.confirm(`Delete provider "${provider.name}"?`)) {
                          remove.mutate(provider.id);
                        }
                      }}
                    >
                      <Trash2 className="h-3.5 w-3.5 text-slate-500" />
                    </Button>
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </CardBody>

      {providerOpen && (
        <AddProviderDialog
          open={providerOpen}
          onOpenChange={setProviderOpen}
          provider={editing}
          onSaved={() => {
            void queryClient.invalidateQueries({ queryKey: ["ai-providers"] });
          }}
        />
      )}
    </Card>
  );
}

function AddProviderDialog({
  open,
  onOpenChange,
  provider,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  provider: AiProviderSummary | null;
  onSaved: () => void;
}) {
  const editing = Boolean(provider);
  const [name, setName] = useState(provider?.name ?? "");
  const [kind, setKind] = useState<AiProviderKind>(provider?.kind ?? "ollama");
  const [baseUrl, setBaseUrl] = useState(provider?.baseUrl ?? "http://localhost:11434");
  const [model, setModel] = useState(provider?.model ?? "");
  const [apiKey, setApiKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [modelsLoaded, setModelsLoaded] = useState(false);
  const [loadingModels, setLoadingModels] = useState(false);
  const [modelError, setModelError] = useState<string | null>(null);

  const save = useMutation({
    mutationFn: (input: AiProviderInput) =>
      provider ? api.updateAiProvider(provider.id, input) : api.createAiProvider(input),
    onSuccess: () => {
      onSaved();
      onOpenChange(false);
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const submit = () => {
    setError(null);
    save.mutate({
      name: name.trim(),
      kind,
      baseUrl: baseUrl.trim(),
      model: model.trim(),
      apiKey: apiKey.trim() ? apiKey.trim() : null,
    });
  };

  const loadModels = async () => {
    setModelError(null);
    setLoadingModels(true);
    try {
      const list = await api.listAiModels(
        kind,
        baseUrl.trim(),
        apiKey.trim() || null,
        provider?.id ?? null,
      );
      setModels(list);
      setModelsLoaded(true);
      if (list.length === 1) {
        setModel(list[0]);
      }
    } catch (mutationError) {
      setModels([]);
      setModelsLoaded(true);
      setModelError(
        mutationError instanceof Error ? mutationError.message : "Unable to load models.",
      );
    } finally {
      setLoadingModels(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setError(null);
        onOpenChange(next);
      }}
      title={editing ? "Edit AI provider" : "Add AI provider"}
      description="Credentials are encrypted at rest. Only metadata is returned to the interface."
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={save.isPending}>
            {save.isPending && <Spinner />}
            Save provider
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <Field label="Name">
          <Input
            autoFocus
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Local Ollama"
          />
        </Field>
        <Field label="Provider type">
          <Select
            value={kind}
            onChange={(event) => {
              const next = event.target.value as AiProviderKind;
              setKind(next);
              setBaseUrl(kindPlaceholder[next]);
              setModels([]);
              setModelsLoaded(false);
              setModelError(null);
            }}
          >
            {(Object.keys(kindLabel) as AiProviderKind[]).map((value) => (
              <option key={value} value={value}>
                {kindLabel[value]}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="Base URL" hint={kindHint[kind]}>
          <Input
            value={baseUrl}
            onChange={(event) => {
              setBaseUrl(event.target.value);
              setModelsLoaded(false);
              setModels([]);
            }}
            placeholder={kindPlaceholder[kind]}
          />
        </Field>
        <Field label="Model" hint="Use the exact model identifier expected by the provider.">
          <div className="flex gap-2">
            <Input
              value={model}
              onChange={(event) => setModel(event.target.value)}
              placeholder="llama3.2"
            />
            <Button
              variant="secondary"
              size="sm"
              className="h-10 shrink-0"
              onClick={() => void loadModels()}
              disabled={loadingModels || !baseUrl.trim() || kind === "azure_openai"}
            >
              {loadingModels ? <Spinner /> : <RefreshCw className="h-3.5 w-3.5" />}
              Load models
            </Button>
          </div>
        </Field>
        {modelsLoaded && models.length > 0 && (
          <Select
            value=""
            onChange={(event) => {
              if (event.target.value) setModel(event.target.value);
            }}
          >
            <option value="">Choose an installed model…</option>
            {models.map((available) => (
              <option key={available} value={available}>
                {available}
              </option>
            ))}
          </Select>
        )}
        {modelsLoaded && models.length === 0 && (
          <p className="rounded-lg border border-amber-500/20 bg-amber-500/5 px-3 py-2 text-[11px] text-amber-200/80">
            {kind === "ollama" ? (
              <>
                No models found. For Ollama, start the service and run{" "}
                <code className="font-mono">ollama pull llama3.2</code> (or another model), then
                load again. A 404 at <code className="font-mono">/api/chat</code> means the model is
                not installed.
              </>
            ) : (
              <>No models returned. Check the provider URL, API key, and model access.</>
            )}
          </p>
        )}
        {modelError && <ErrorBlock message={modelError} />}
        <Field
          label="API key"
          hint={
            editing
              ? "Leave blank to keep the stored key. A new value replaces it."
              : "Optional for local Ollama. Stored encrypted; never shown again."
          }
        >
          <Input
            type="password"
            autoComplete="off"
            value={apiKey}
            onChange={(event) => setApiKey(event.target.value)}
          />
        </Field>
        <div className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[11px] text-amber-200/80">
          <Bot className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          Cloud providers are blocked per engagement unless the AI policy is set to approved cloud.
        </div>
        {error && <ErrorBlock message={error} />}
      </div>
    </Dialog>
  );
}
