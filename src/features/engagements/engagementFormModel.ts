import { z } from "zod";
import type { EngagementDetail, EngagementInput } from "@/types";

const scopeAssetSchema = z.object({
  kind: z.string().min(1),
  value: z.string().trim().min(1, "Enter a value."),
  inScope: z.boolean(),
  notes: z.string(),
});

const riskSchema = z.object({
  title: z.string().trim().min(1, "Enter a risk title.").max(200),
  description: z.string(),
  likelihood: z.number().int().min(0).max(9),
  impact: z.number().int().min(0).max(9),
  notes: z.string(),
});

const contactSchema = z.object({
  name: z.string().trim().min(1, "Enter a contact name.").max(200),
  role: z.string(),
  email: z.union([z.literal(""), z.string().email("Enter a valid email.")]),
  phone: z.string(),
});

const assumptionSchema = z.object({
  text: z.string().trim().min(1, "Enter an assumption."),
});

export const engagementFormSchema = z
  .object({
    name: z.string().trim().min(1, "Enter an engagement name.").max(200),
    clientCode: z.string().max(120),
    status: z.enum(["draft", "active", "in_review", "completed", "archived"]),
    startDate: z.string(),
    endDate: z.string(),
    applicationSummary: z.string().max(20_000),
    attackSurface: z.string().max(20_000),
    scopeAssets: z.array(scopeAssetSchema),
    risks: z.array(riskSchema),
    assumptions: z.array(assumptionSchema),
    contacts: z.array(contactSchema),
  })
  .refine(
    (values) =>
      !values.startDate ||
      !values.endDate ||
      new Date(values.startDate) <= new Date(values.endDate),
    { path: ["endDate"], message: "End date must be on or after the start date." },
  );

export type EngagementFormValues = z.infer<typeof engagementFormSchema>;

export function emptyEngagementValues(): EngagementFormValues {
  return {
    name: "",
    clientCode: "",
    status: "draft",
    startDate: "",
    endDate: "",
    applicationSummary: "",
    attackSurface: "",
    scopeAssets: [],
    risks: [],
    assumptions: [],
    contacts: [],
  };
}

export function valuesFromDetail(detail: EngagementDetail): EngagementFormValues {
  const { engagement } = detail;
  return {
    name: engagement.name,
    clientCode: engagement.clientCode,
    status: engagement.status,
    startDate: engagement.startDate,
    endDate: engagement.endDate,
    applicationSummary: engagement.applicationSummary,
    attackSurface: engagement.attackSurface,
    scopeAssets: detail.scopeAssets.map((asset) => ({
      kind: asset.kind,
      value: asset.value,
      inScope: asset.inScope,
      notes: asset.notes,
    })),
    risks: detail.risks.map((risk) => ({
      title: risk.title,
      description: risk.description,
      likelihood: risk.likelihood,
      impact: risk.impact,
      notes: risk.notes,
    })),
    assumptions: detail.assumptions.map((assumption) => ({ text: assumption.text })),
    contacts: detail.contacts.map((contact) => ({
      name: contact.name,
      role: contact.role,
      email: contact.email,
      phone: contact.phone,
    })),
  };
}

export function toEngagementInput(values: EngagementFormValues): EngagementInput {
  return {
    name: values.name.trim(),
    clientCode: values.clientCode.trim(),
    status: values.status,
    startDate: values.startDate,
    endDate: values.endDate,
    applicationSummary: values.applicationSummary,
    attackSurface: values.attackSurface,
    scopeAssets: values.scopeAssets.map((asset) => ({
      ...asset,
      kind: asset.kind.trim(),
      value: asset.value.trim(),
    })),
    risks: values.risks,
    assumptions: values.assumptions.map((assumption) => assumption.text.trim()),
    contacts: values.contacts.map((contact) => ({
      name: contact.name.trim(),
      role: contact.role.trim(),
      email: contact.email.trim(),
      phone: contact.phone.trim(),
    })),
  };
}
