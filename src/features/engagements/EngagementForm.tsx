import { Controller, useFieldArray, useForm, useWatch } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Field, Input, Textarea } from "@/components/ui/input";
import { Select, SelectItem } from "@/components/ui/select";
import { SeverityBadge } from "@/components/ui/badge";
import { SectionTitle } from "@/components/layout/PageHeader";
import { severityFor } from "@/lib/severity";
import { scopeKinds } from "@/lib/labels";
import type { EngagementInput } from "@/types";
import {
  engagementFormSchema,
  toEngagementInput,
  type EngagementFormValues,
} from "@/features/engagements/engagementFormModel";

export function EngagementForm({
  formId,
  defaultValues,
  onSubmit,
  submitting,
  error,
}: {
  formId: string;
  defaultValues: EngagementFormValues;
  onSubmit: (input: EngagementInput) => void;
  submitting?: boolean;
  error?: string | null;
}) {
  const form = useForm<EngagementFormValues>({
    resolver: zodResolver(engagementFormSchema),
    defaultValues,
    mode: "onBlur",
  });
  const scope = useFieldArray({ control: form.control, name: "scopeAssets" });
  const risks = useFieldArray({ control: form.control, name: "risks" });
  const assumptions = useFieldArray({ control: form.control, name: "assumptions" });
  const contacts = useFieldArray({ control: form.control, name: "contacts" });

  const riskValues = useWatch({ control: form.control, name: "risks" });

  return (
    <form
      id={formId}
      className="space-y-8"
      onSubmit={form.handleSubmit((values) => onSubmit(toEngagementInput(values)))}
    >
      <section className="space-y-4">
        <SectionTitle>Engagement</SectionTitle>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="Project name" error={form.formState.errors.name?.message}>
            <Input placeholder="Customer portal assessment" {...form.register("name")} />
          </Field>
          <Field label="Client code" hint="Short reference; avoid full client names.">
            <Input placeholder="CLIENT-2026-001" {...form.register("clientCode")} />
          </Field>
          <Field label="Status">
            <Controller
              control={form.control}
              name="status"
              render={({ field }) => (
                <Select value={field.value} onValueChange={field.onChange}>
                  <SelectItem value="draft">Draft</SelectItem>
                  <SelectItem value="active">Active</SelectItem>
                  <SelectItem value="in_review">In review</SelectItem>
                  <SelectItem value="completed">Completed</SelectItem>
                  <SelectItem value="archived">Archived</SelectItem>
                </Select>
              )}
            />
          </Field>
          <div className="grid grid-cols-2 gap-3">
            <Field label="Start date">
              <Input type="date" {...form.register("startDate")} />
            </Field>
            <Field label="End date" error={form.formState.errors.endDate?.message}>
              <Input type="date" {...form.register("endDate")} />
            </Field>
          </div>
        </div>
        <Field label="Application summary">
          <Textarea
            rows={3}
            placeholder="What the application does, technology, users, and business context."
            {...form.register("applicationSummary")}
          />
        </Field>
        <Field label="Attack surface">
          <Textarea
            rows={3}
            placeholder="Authentication, roles, APIs, uploads, admin functions, integrations, hosting."
            {...form.register("attackSurface")}
          />
        </Field>
      </section>

      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <SectionTitle>Scope assets</SectionTitle>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => scope.append({ kind: "url", value: "", inScope: true, notes: "" })}
          >
            <Plus className="h-3.5 w-3.5" /> Add asset
          </Button>
        </div>
        {scope.fields.length === 0 && (
          <p className="rounded-lg border border-dashed border-slate-800 px-4 py-6 text-center text-xs text-slate-500">
            No scope assets yet. Add URLs, hosts, APIs, or cloud accounts.
          </p>
        )}
        {scope.fields.map((field, index) => (
          <div
            key={field.id}
            className="grid gap-3 rounded-lg border border-slate-800 bg-slate-900/40 p-3 sm:grid-cols-[130px_1fr_auto_1fr_auto] sm:items-end"
          >
            <Field label="Kind">
              <Controller
                control={form.control}
                name={`scopeAssets.${index}.kind`}
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
                    {scopeKinds.map((kind) => (
                      <SelectItem key={kind} value={kind}>
                        {kind.replace("_", " ")}
                      </SelectItem>
                    ))}
                  </Select>
                )}
              />
            </Field>
            <Field label="Value" error={form.formState.errors.scopeAssets?.[index]?.value?.message}>
              <Input
                placeholder="https://app.example.test"
                {...form.register(`scopeAssets.${index}.value`)}
              />
            </Field>
            <label className="flex h-10 items-center gap-2 text-xs text-slate-300">
              <input
                type="checkbox"
                className="h-4 w-4 rounded border-slate-600 bg-slate-900"
                {...form.register(`scopeAssets.${index}.inScope`)}
              />
              In scope
            </label>
            <Field label="Notes">
              <Input placeholder="Optional" {...form.register(`scopeAssets.${index}.notes`)} />
            </Field>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Remove asset"
              onClick={() => scope.remove(index)}
            >
              <Trash2 className="h-4 w-4 text-slate-500" />
            </Button>
          </div>
        ))}
      </section>

      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <SectionTitle>Risks</SectionTitle>
          <Button
            variant="secondary"
            size="sm"
            onClick={() =>
              risks.append({
                title: "",
                description: "",
                likelihood: 5,
                impact: 5,
                notes: "",
              })
            }
          >
            <Plus className="h-3.5 w-3.5" /> Add risk
          </Button>
        </div>
        {risks.fields.length === 0 && (
          <p className="rounded-lg border border-dashed border-slate-800 px-4 py-6 text-center text-xs text-slate-500">
            Capture business and technical risks identified during scoping.
          </p>
        )}
        {risks.fields.map((field, index) => {
          const rawLikelihood = Number(riskValues?.[index]?.likelihood ?? 0);
          const rawImpact = Number(riskValues?.[index]?.impact ?? 0);
          const likelihood = Number.isFinite(rawLikelihood) ? rawLikelihood : 0;
          const impact = Number.isFinite(rawImpact) ? rawImpact : 0;
          const severity = severityFor(
            Math.min(9, Math.max(0, likelihood)),
            Math.min(9, Math.max(0, impact)),
          );
          return (
            <div
              key={field.id}
              className="space-y-3 rounded-lg border border-slate-800 bg-slate-900/40 p-3"
            >
              <div className="grid gap-3 sm:grid-cols-[1fr_110px_110px_auto_auto] sm:items-end">
                <Field label="Risk" error={form.formState.errors.risks?.[index]?.title?.message}>
                  <Input
                    placeholder="Unauthorized access to customer data"
                    {...form.register(`risks.${index}.title`)}
                  />
                </Field>
                <Field label="Likelihood">
                  <Input
                    type="number"
                    min={0}
                    max={9}
                    {...form.register(`risks.${index}.likelihood`, { valueAsNumber: true })}
                  />
                </Field>
                <Field label="Impact">
                  <Input
                    type="number"
                    min={0}
                    max={9}
                    {...form.register(`risks.${index}.impact`, { valueAsNumber: true })}
                  />
                </Field>
                <div className="flex h-10 items-center">
                  <SeverityBadge severity={severity} />
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="Remove risk"
                  onClick={() => risks.remove(index)}
                >
                  <Trash2 className="h-4 w-4 text-slate-500" />
                </Button>
              </div>
              <Field label="Description">
                <Input
                  placeholder="Why it matters"
                  {...form.register(`risks.${index}.description`)}
                />
              </Field>
            </div>
          );
        })}
      </section>

      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <SectionTitle>Assumptions</SectionTitle>
          <Button variant="secondary" size="sm" onClick={() => assumptions.append({ text: "" })}>
            <Plus className="h-3.5 w-3.5" /> Add assumption
          </Button>
        </div>
        {assumptions.fields.map((field, index) => (
          <div key={field.id} className="flex items-end gap-2">
            <Field
              label={`Assumption ${index + 1}`}
              className="flex-1"
              error={form.formState.errors.assumptions?.[index]?.text?.message}
            >
              <Input
                placeholder="Testing is limited to the listed scope assets."
                {...form.register(`assumptions.${index}.text`)}
              />
            </Field>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Remove assumption"
              onClick={() => assumptions.remove(index)}
            >
              <Trash2 className="h-4 w-4 text-slate-500" />
            </Button>
          </div>
        ))}
      </section>

      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <SectionTitle>Contacts</SectionTitle>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => contacts.append({ name: "", role: "", email: "", phone: "" })}
          >
            <Plus className="h-3.5 w-3.5" /> Add contact
          </Button>
        </div>
        {contacts.fields.map((field, index) => (
          <div
            key={field.id}
            className="grid gap-3 rounded-lg border border-slate-800 bg-slate-900/40 p-3 sm:grid-cols-[1fr_1fr_1fr_1fr_auto] sm:items-end"
          >
            <Field label="Name" error={form.formState.errors.contacts?.[index]?.name?.message}>
              <Input {...form.register(`contacts.${index}.name`)} />
            </Field>
            <Field label="Role">
              <Input {...form.register(`contacts.${index}.role`)} />
            </Field>
            <Field label="Email" error={form.formState.errors.contacts?.[index]?.email?.message}>
              <Input type="email" {...form.register(`contacts.${index}.email`)} />
            </Field>
            <Field label="Phone">
              <Input {...form.register(`contacts.${index}.phone`)} />
            </Field>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Remove contact"
              onClick={() => contacts.remove(index)}
            >
              <Trash2 className="h-4 w-4 text-slate-500" />
            </Button>
          </div>
        ))}
      </section>

      {error && (
        <p className="rounded-lg border border-red-500/30 bg-red-500/5 px-3 py-2 text-xs text-red-300">
          {error}
        </p>
      )}

      <button type="submit" className="hidden" disabled={submitting}>
        Save
      </button>
    </form>
  );
}
