import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound, Package, Power, ShieldCheck, Trash2, Upload } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { Field, Input } from "@/components/ui/input";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { useSession } from "@/hooks/useSession";
import { api } from "@/lib/api";
import { readFileAsBase64 } from "@/lib/base64";
import type { AddPluginPublisherInput } from "@/types";

const MAX_PLUGIN_BYTES = 8 * 1024 * 1024;

export function PluginsCard() {
  const queryClient = useQueryClient();
  const session = useSession();
  const canManage =
    session.data?.profile?.role === "admin" || session.data?.profile?.role === "lead";
  const [publisherOpen, setPublisherOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installFile, setInstallFile] = useState<File | null>(null);
  const [installing, setInstalling] = useState(false);

  const publishers = useQuery({
    queryKey: ["plugin-publishers"],
    queryFn: api.listPluginPublishers,
  });
  const plugins = useQuery({ queryKey: ["plugins"], queryFn: api.listPlugins });

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["plugin-publishers"] });
    void queryClient.invalidateQueries({ queryKey: ["plugins"] });
  };

  const removePublisher = useMutation({
    mutationFn: (id: string) => api.deletePluginPublisher(id),
    onSuccess: invalidate,
    onError: (mutationError) => setError(mutationError.message),
  });

  const setEnabled = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      api.setPluginEnabled(id, enabled),
    onSuccess: invalidate,
    onError: (mutationError) => setError(mutationError.message),
  });

  const removePlugin = useMutation({
    mutationFn: (id: string) => api.deletePlugin(id),
    onSuccess: invalidate,
    onError: (mutationError) => setError(mutationError.message),
  });

  const install = useMutation({
    mutationFn: (dataBase64: string) => api.installPlugin(dataBase64),
    onSuccess: () => {
      setInstallFile(null);
      invalidate();
    },
    onError: (mutationError) => setError(mutationError.message),
  });

  const installSelected = async () => {
    setError(null);
    if (!installFile) {
      setError("Choose a .ttplugin package.");
      return;
    }
    if (installFile.size > MAX_PLUGIN_BYTES) {
      setError("Plugin packages must be 8 MB or smaller.");
      return;
    }
    setInstalling(true);
    try {
      const data = await readFileAsBase64(installFile);
      install.mutate(data);
    } catch (readError) {
      setError(readError instanceof Error ? readError.message : "Unable to read the package.");
    } finally {
      setInstalling(false);
    }
  };

  return (
    <Card className="mt-4">
      <CardHeader
        title="Plugins"
        description="Signed WebAssembly modules with declared capabilities, executed in a resource-limited sandbox."
        action={
          canManage ? (
            <Button size="sm" variant="secondary" onClick={() => setPublisherOpen(true)}>
              <KeyRound className="h-3.5 w-3.5" /> Add publisher key
            </Button>
          ) : undefined
        }
      />
      <CardBody className="space-y-6">
        {error && <ErrorBlock message={error} />}

        <section>
          <div className="mb-2 flex items-center gap-2">
            <ShieldCheck className="h-4 w-4 text-emerald-400" />
            <h3 className="text-xs font-semibold uppercase tracking-wide text-slate-400">
              Trusted publishers
            </h3>
          </div>
          {publishers.isLoading && <LoadingBlock label="Loading publishers…" />}
          {publishers.isError && <ErrorBlock message={publishers.error.message} />}
          {publishers.data && publishers.data.length === 0 && (
            <p className="rounded-lg border border-dashed border-slate-800 px-4 py-4 text-xs text-slate-500">
              No publisher keys yet. Import an Ed25519 public key before installing a signed plugin.
            </p>
          )}
          {publishers.data && publishers.data.length > 0 && (
            <div className="overflow-hidden rounded-lg border border-slate-800">
              <table className="w-full text-left text-xs">
                <thead className="border-b border-slate-800 bg-slate-950/40 text-[10px] uppercase tracking-wide text-slate-500">
                  <tr>
                    <th className="px-3 py-2 font-medium">Publisher</th>
                    <th className="px-3 py-2 font-medium">Public key</th>
                    <th className="px-3 py-2 text-right font-medium">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-800">
                  {publishers.data.map((publisher) => (
                    <tr key={publisher.id}>
                      <td className="px-3 py-2">
                        <p className="font-medium text-slate-200">{publisher.name}</p>
                        <p className="text-[10px] text-slate-500">{publisher.id}</p>
                      </td>
                      <td className="px-3 py-2 font-mono text-[10px] text-slate-500">
                        {publisher.publicKey.slice(0, 20)}…
                      </td>
                      <td className="px-3 py-2 text-right">
                        {canManage && (
                          <Button
                            variant="ghost"
                            size="icon"
                            aria-label="Delete publisher key"
                            onClick={() => {
                              if (window.confirm(`Delete publisher "${publisher.name}"?`)) {
                                removePublisher.mutate(publisher.id);
                              }
                            }}
                          >
                            <Trash2 className="h-3.5 w-3.5 text-slate-500" />
                          </Button>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>

        <section>
          <div className="mb-2 flex items-center gap-2">
            <Package className="h-4 w-4 text-emerald-400" />
            <h3 className="text-xs font-semibold uppercase tracking-wide text-slate-400">
              Installed plugins
            </h3>
          </div>

          {canManage && (
            <div className="mb-3 flex flex-wrap items-center gap-2 rounded-lg border border-slate-800 bg-slate-950/40 p-3">
              <input
                type="file"
                accept=".ttplugin"
                onChange={(event) => {
                  setInstallFile(event.target.files?.[0] ?? null);
                  setError(null);
                }}
                className="min-w-0 flex-1 cursor-pointer text-xs text-slate-300 file:mr-3 file:rounded-md file:border-0 file:bg-slate-800 file:px-3 file:py-1.5 file:text-xs file:text-slate-200"
              />
              <Button
                size="sm"
                onClick={() => void installSelected()}
                disabled={!installFile || installing || install.isPending}
              >
                {installing || install.isPending ? <Spinner /> : <Upload className="h-3.5 w-3.5" />}
                Install
              </Button>
            </div>
          )}

          {plugins.isLoading && <LoadingBlock label="Loading plugins…" />}
          {plugins.isError && <ErrorBlock message={plugins.error.message} />}
          {plugins.data && plugins.data.length === 0 && (
            <EmptyState
              title="No plugins installed"
              description="Install a signed .ttplugin package from a trusted publisher."
            />
          )}
          {plugins.data && plugins.data.length > 0 && (
            <ul className="divide-y divide-slate-800 rounded-lg border border-slate-800">
              {plugins.data.map((plugin) => (
                <li
                  key={plugin.id}
                  className="flex flex-wrap items-center justify-between gap-3 px-3 py-3"
                >
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="text-xs font-medium text-slate-200">{plugin.name}</p>
                      <Badge tone={plugin.enabled ? "success" : "neutral"}>
                        {plugin.enabled ? "Enabled" : "Disabled"}
                      </Badge>
                      <Badge>v{plugin.version}</Badge>
                    </div>
                    <p className="mt-0.5 text-[10px] text-slate-500">
                      {plugin.id} · {plugin.publisherId}
                    </p>
                    <div className="mt-1 flex flex-wrap gap-1">
                      {plugin.capabilities.length === 0 && <Badge>No engagement access</Badge>}
                      {plugin.capabilities.map((capability) => (
                        <Badge key={capability} tone="accent">
                          {capability}
                        </Badge>
                      ))}
                    </div>
                  </div>
                  {canManage && (
                    <div className="flex gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={plugin.enabled ? "Disable plugin" : "Enable plugin"}
                        onClick={() =>
                          setEnabled.mutate({ id: plugin.id, enabled: !plugin.enabled })
                        }
                      >
                        <Power
                          className={
                            plugin.enabled
                              ? "h-3.5 w-3.5 text-emerald-400"
                              : "h-3.5 w-3.5 text-slate-500"
                          }
                        />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label="Delete plugin"
                        onClick={() => {
                          if (window.confirm(`Delete plugin "${plugin.name}"?`)) {
                            removePlugin.mutate(plugin.id);
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
        </section>
      </CardBody>

      {publisherOpen && (
        <AddPublisherDialog
          open={publisherOpen}
          onOpenChange={setPublisherOpen}
          onSaved={invalidate}
        />
      )}
    </Card>
  );
}

function AddPublisherDialog({
  open,
  onOpenChange,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: () => void;
}) {
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [publicKey, setPublicKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const add = useMutation({
    mutationFn: (input: AddPluginPublisherInput) => api.addPluginPublisher(input),
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
      title="Add publisher key"
      description="Only packages signed by a trusted Ed25519 public key can be installed."
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            onClick={() =>
              add.mutate({
                id: id.trim(),
                name: name.trim(),
                publicKey: publicKey.trim(),
              })
            }
            disabled={add.isPending}
          >
            {add.isPending && <Spinner />}
            Save publisher
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <Field label="Publisher ID" hint="Must match the publisherId in the plugin manifest.">
          <Input
            autoFocus
            value={id}
            onChange={(event) => setId(event.target.value)}
            placeholder="com.example.publisher"
          />
        </Field>
        <Field label="Display name">
          <Input value={name} onChange={(event) => setName(event.target.value)} />
        </Field>
        <Field label="Public key" hint="Ed25519 public key in hex, 64 characters.">
          <Input
            value={publicKey}
            onChange={(event) => setPublicKey(event.target.value)}
            placeholder="64 hex characters"
            className="font-mono text-xs"
          />
        </Field>
        {error && <ErrorBlock message={error} />}
      </div>
    </Dialog>
  );
}
