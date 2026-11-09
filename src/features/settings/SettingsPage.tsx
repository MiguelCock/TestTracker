import { useMutation } from "@tanstack/react-query";
import {
  CheckCircle2,
  Database,
  KeyRound,
  Lock,
  ScrollText,
  ShieldCheck,
  XCircle,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { PageHeader } from "@/components/layout/PageHeader";
import { ErrorBlock, Spinner } from "@/components/ui/states";
import { AiProvidersCard } from "@/features/settings/AiProvidersCard";
import { McpCard } from "@/features/settings/McpCard";
import { PluginsCard } from "@/features/settings/PluginsCard";
import { useLockVault, useSession } from "@/hooks/useSession";
import { api } from "@/lib/api";
import { initials } from "@/lib/format";
import { roleLabel } from "@/lib/labels";

export function SettingsPage() {
  const session = useSession();
  const lock = useLockVault();
  const verify = useMutation({ mutationFn: api.verifyAuditChain });
  const profile = session.data?.profile;

  return (
    <div>
      <PageHeader
        title="Settings"
        description="Local profile, security posture, and vault controls."
      />

      <div className="grid gap-4 xl:grid-cols-2">
        <Card>
          <CardHeader title="Local profile" description="This device's signing identity." />
          <CardBody>
            <div className="flex items-center gap-3">
              <div className="flex h-11 w-11 items-center justify-center rounded-full bg-slate-800 text-sm font-semibold text-slate-200">
                {profile ? initials(profile.displayName) : "?"}
              </div>
              <div>
                <p className="text-sm font-medium text-slate-100">
                  {profile?.displayName ?? "Unknown"}
                </p>
                <p className="text-xs text-slate-500">
                  {profile ? roleLabel[profile.role] : "No profile"}
                </p>
              </div>
              <Badge tone="success" className="ml-auto">
                Unlocked
              </Badge>
            </div>
            <div className="mt-4 rounded-lg border border-slate-800 bg-slate-950/40 p-3 text-xs text-slate-500">
              Local-first trust model: this device can read its own vault. Cryptographic sign-offs
              and the audit chain make changes verifiable when engagements are shared.
            </div>
            <div className="mt-4 flex items-center justify-between rounded-lg border border-slate-800 bg-slate-950/40 px-3 py-2">
              <div>
                <p className="text-xs font-medium text-slate-300">Auto-lock</p>
                <p className="text-[11px] text-slate-500">
                  Clears keys from memory after inactivity.
                </p>
              </div>
              <span className="text-xs text-slate-300">
                {Math.round((session.data?.autoLockSeconds ?? 0) / 60)} min
              </span>
            </div>
            <Button
              variant="secondary"
              className="mt-4 w-full"
              onClick={() => lock.mutate()}
              disabled={lock.isPending}
            >
              <Lock className="h-4 w-4" /> Lock vault now
            </Button>
          </CardBody>
        </Card>

        <Card>
          <CardHeader
            title="Security posture"
            description="Controls enabled in this build."
            action={
              <Button
                variant="secondary"
                size="sm"
                onClick={() => verify.mutate()}
                disabled={verify.isPending}
              >
                {verify.isPending ? <Spinner /> : <ScrollText className="h-3.5 w-3.5" />}
                Verify audit chain
              </Button>
            }
          />
          <CardBody className="space-y-3">
            <SecurityRow
              icon={<Database className="h-4 w-4" />}
              title="Encrypted database"
              detail="SQLCipher with a key derived from the vault master key."
            />
            <SecurityRow
              icon={<KeyRound className="h-4 w-4" />}
              title="Argon2id passphrase"
              detail="Memory-hard KDF; the passphrase never reaches the webview."
            />
            <SecurityRow
              icon={<ShieldCheck className="h-4 w-4" />}
              title="Tamper-evident audit"
              detail="Append-only hash chain and Ed25519 sign-offs verified against snapshot hashes."
            />
            <SecurityRow
              icon={<ScrollText className="h-4 w-4" />}
              title="Least-privilege IPC"
              detail="Minimal Tauri capabilities; no remote content or shell access."
            />

            {verify.data === true && (
              <div className="flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-xs text-emerald-300">
                <CheckCircle2 className="h-4 w-4" /> Audit chain verified.
              </div>
            )}
            {verify.data === false && (
              <div className="flex items-center gap-2 rounded-lg border border-red-500/30 bg-red-500/5 px-3 py-2 text-xs text-red-300">
                <XCircle className="h-4 w-4" /> Audit chain verification failed.
              </div>
            )}
            {verify.isError && <ErrorBlock message={verify.error.message} />}
          </CardBody>
        </Card>
      </div>

      <AiProvidersCard />

      <McpCard />

      <PluginsCard />

      <Card className="mt-4">
        <CardBody className="flex flex-wrap items-center justify-between gap-3 text-xs text-slate-500">
          <span>TestTracker 0.1.0 · local-first desktop application</span>
          <span>No telemetry. AI calls are explicit and policy-controlled.</span>
        </CardBody>
      </Card>
    </div>
  );
}

function SecurityRow({
  icon,
  title,
  detail,
}: {
  icon: React.ReactNode;
  title: string;
  detail: string;
}) {
  return (
    <div className="flex items-start gap-3 rounded-lg border border-slate-800 bg-slate-950/40 p-3">
      <div className="mt-0.5 text-emerald-400">{icon}</div>
      <div>
        <p className="text-xs font-medium text-slate-200">{title}</p>
        <p className="mt-0.5 text-[11px] leading-relaxed text-slate-500">{detail}</p>
      </div>
      <CheckCircle2 className="ml-auto h-3.5 w-3.5 shrink-0 text-emerald-500/70" />
    </div>
  );
}
