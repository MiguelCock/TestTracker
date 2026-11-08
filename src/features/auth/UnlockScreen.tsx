import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";
import { Fingerprint, KeyRound, LockKeyhole, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Field, Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import type { SessionInfo } from "@/types";

export function UnlockScreen({ initialized }: { initialized: boolean }) {
  return (
    <div className="grid min-h-full grid-cols-1 lg:grid-cols-[1.1fr_1fr]">
      <section className="relative hidden overflow-hidden border-r border-slate-800 bg-slate-950 lg:flex lg:flex-col lg:justify-between lg:p-12">
        <div className="absolute -left-24 top-24 h-72 w-72 rounded-full bg-emerald-500/10 blur-3xl" />
        <div className="absolute -right-16 bottom-10 h-80 w-80 rounded-full bg-sky-500/10 blur-3xl" />
        <div className="relative">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-500/15 ring-1 ring-emerald-500/30">
              <ShieldCheck className="h-5 w-5 text-emerald-400" />
            </div>
            <div>
              <p className="text-sm font-semibold tracking-wide text-white">TestTracker</p>
              <p className="text-xs text-slate-500">Secure engagement workspace</p>
            </div>
          </div>

          <h1 className="mt-16 max-w-md text-4xl font-semibold leading-tight text-white">
            Plan, execute, and report penetration tests without leaving evidence behind.
          </h1>
          <p className="mt-4 max-w-md text-sm leading-relaxed text-slate-400">
            Engagements, test cases, findings, evidence, approvals, and audit history in one
            local-first encrypted workspace.
          </p>

          <div className="mt-10 grid max-w-md gap-3">
            <Feature icon={<LockKeyhole className="h-4 w-4" />} title="SQLCipher database">
              AES-256 encryption at rest with a key derived from your passphrase.
            </Feature>
            <Feature icon={<KeyRound className="h-4 w-4" />} title="Argon2id key derivation">
              Memory-hard passphrase protection; secrets never touch the webview.
            </Feature>
            <Feature icon={<Fingerprint className="h-4 w-4" />} title="Tamper-evident audit">
              Hash-chained audit events and a per-profile signing identity for approvals.
            </Feature>
          </div>
        </div>
        <p className="relative text-xs text-slate-600">
          The passphrase and encryption keys never leave this device.
        </p>
      </section>

      <section className="flex items-center justify-center px-5 py-12">
        <div className="w-full max-w-sm">
          <div className="mb-8 flex items-center gap-3 lg:hidden">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-500/15 ring-1 ring-emerald-500/30">
              <ShieldCheck className="h-5 w-5 text-emerald-400" />
            </div>
            <p className="text-sm font-semibold text-white">TestTracker</p>
          </div>
          {initialized ? <UnlockForm /> : <SetupForm />}
        </div>
      </section>
    </div>
  );
}

function Feature({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-start gap-3 rounded-lg border border-slate-800/80 bg-slate-900/40 p-3">
      <div className="mt-0.5 text-emerald-400">{icon}</div>
      <div>
        <p className="text-xs font-medium text-slate-200">{title}</p>
        <p className="mt-0.5 text-xs leading-relaxed text-slate-500">{children}</p>
      </div>
    </div>
  );
}

const setupSchema = z
  .object({
    displayName: z.string().trim().min(1, "Enter your name.").max(120, "Name is too long."),
    passphrase: z.string().min(12, "Use at least 12 characters."),
    confirm: z.string(),
  })
  .refine((values) => values.passphrase === values.confirm, {
    path: ["confirm"],
    message: "Passphrases do not match.",
  });

type SetupValues = z.infer<typeof setupSchema>;

function SetupForm() {
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<string | null>(null);
  const form = useForm<SetupValues>({
    resolver: zodResolver(setupSchema),
    defaultValues: { displayName: "", passphrase: "", confirm: "" },
  });

  const mutation = useMutation({
    mutationFn: (values: SetupValues) => api.initializeVault(values.passphrase, values.displayName),
    onSuccess: (info: SessionInfo) => {
      queryClient.setQueryData(["session"], info);
    },
    onError: (error) => setServerError(error.message),
  });

  return (
    <div>
      <p className="text-xs font-semibold uppercase tracking-[0.2em] text-emerald-400">First run</p>
      <h2 className="mt-2 text-2xl font-semibold text-white">Create your encrypted vault</h2>
      <p className="mt-2 text-sm leading-relaxed text-slate-400">
        Choose a passphrase that protects every engagement on this device. It cannot be recovered if
        lost.
      </p>

      <form
        className="mt-6 space-y-4"
        onSubmit={form.handleSubmit((values) => {
          setServerError(null);
          mutation.mutate(values);
        })}
      >
        <Field label="Display name" error={form.formState.errors.displayName?.message}>
          <Input
            autoFocus
            autoComplete="name"
            placeholder="Alex Tester"
            {...form.register("displayName")}
          />
        </Field>
        <Field
          label="Passphrase"
          hint="At least 12 characters. A long phrase is strongest."
          error={form.formState.errors.passphrase?.message}
        >
          <Input type="password" autoComplete="new-password" {...form.register("passphrase")} />
        </Field>
        <Field label="Confirm passphrase" error={form.formState.errors.confirm?.message}>
          <Input type="password" autoComplete="new-password" {...form.register("confirm")} />
        </Field>

        {serverError && (
          <p className="rounded-lg border border-red-500/30 bg-red-500/5 px-3 py-2 text-xs text-red-300">
            {serverError}
          </p>
        )}

        <Button type="submit" className="w-full" disabled={mutation.isPending}>
          {mutation.isPending ? <Spinner /> : <LockKeyhole className="h-4 w-4" />}
          Create secure vault
        </Button>
      </form>
    </div>
  );
}

const unlockSchema = z.object({
  passphrase: z.string().min(1, "Enter your passphrase."),
});

type UnlockValues = z.infer<typeof unlockSchema>;

function UnlockForm() {
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<string | null>(null);
  const form = useForm<UnlockValues>({
    resolver: zodResolver(unlockSchema),
    defaultValues: { passphrase: "" },
  });

  const mutation = useMutation({
    mutationFn: (values: UnlockValues) => api.unlockVault(values.passphrase),
    onSuccess: (info) => {
      queryClient.setQueryData(["session"], info);
    },
    onError: (error) => setServerError(error.message),
  });

  return (
    <div>
      <p className="text-xs font-semibold uppercase tracking-[0.2em] text-emerald-400">Locked</p>
      <h2 className="mt-2 text-2xl font-semibold text-white">Unlock TestTracker</h2>
      <p className="mt-2 text-sm leading-relaxed text-slate-400">
        Enter your passphrase to open the encrypted engagement store on this device.
      </p>

      <form
        className="mt-6 space-y-4"
        onSubmit={form.handleSubmit((values) => {
          setServerError(null);
          mutation.mutate(values);
        })}
      >
        <Field label="Passphrase" error={form.formState.errors.passphrase?.message}>
          <Input
            autoFocus
            type="password"
            autoComplete="current-password"
            {...form.register("passphrase")}
          />
        </Field>

        {serverError && (
          <p className="rounded-lg border border-red-500/30 bg-red-500/5 px-3 py-2 text-xs text-red-300">
            {serverError}
          </p>
        )}

        <Button type="submit" className="w-full" disabled={mutation.isPending}>
          {mutation.isPending ? <Spinner /> : <KeyRound className="h-4 w-4" />}
          Unlock vault
        </Button>
      </form>

      <div className="mt-6 flex items-start gap-2 rounded-lg border border-slate-800 bg-slate-900/40 p-3 text-xs text-slate-500">
        <LockKeyhole className="mt-0.5 h-3.5 w-3.5 shrink-0" />
        The vault locks automatically after a period of inactivity.
      </div>
    </div>
  );
}
