import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import {
  engagementStatusLabel,
  findingStatusLabel,
  severityLabel,
  testCaseStatusLabel,
} from "@/lib/labels";
import type { EngagementStatus, FindingStatus, Severity, TestCaseStatus } from "@/types";

type Tone = "neutral" | "info" | "success" | "warning" | "danger" | "accent";

const tones: Record<Tone, string> = {
  neutral: "bg-slate-500/10 text-slate-300 ring-slate-500/25",
  info: "bg-sky-500/10 text-sky-300 ring-sky-500/25",
  success: "bg-emerald-500/10 text-emerald-300 ring-emerald-500/25",
  warning: "bg-amber-500/10 text-amber-300 ring-amber-500/25",
  danger: "bg-red-500/10 text-red-300 ring-red-500/25",
  accent: "bg-violet-500/10 text-violet-300 ring-violet-500/25",
};

export function Badge({
  tone = "neutral",
  className,
  children,
}: {
  tone?: Tone;
  className?: string;
  children: ReactNode;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-medium ring-1 ring-inset",
        tones[tone],
        className,
      )}
    >
      {children}
    </span>
  );
}

const severityTone: Record<Severity, Tone> = {
  informational: "neutral",
  low: "info",
  medium: "warning",
  high: "danger",
  critical: "danger",
};

export function SeverityBadge({ severity }: { severity: Severity }) {
  return <Badge tone={severityTone[severity]}>{severityLabel[severity]}</Badge>;
}

const engagementTone: Record<EngagementStatus, Tone> = {
  draft: "neutral",
  active: "success",
  in_review: "warning",
  completed: "info",
  archived: "neutral",
};

export function EngagementStatusBadge({ status }: { status: EngagementStatus }) {
  return <Badge tone={engagementTone[status]}>{engagementStatusLabel[status]}</Badge>;
}

const testTone: Record<TestCaseStatus, Tone> = {
  not_started: "neutral",
  in_progress: "info",
  done: "success",
  blocked: "danger",
  not_applicable: "neutral",
};

export function TestCaseStatusBadge({ status }: { status: TestCaseStatus }) {
  return <Badge tone={testTone[status]}>{testCaseStatusLabel[status]}</Badge>;
}

const findingTone: Record<FindingStatus, Tone> = {
  open: "danger",
  in_review: "warning",
  remediated: "info",
  retest_pending: "warning",
  closed: "success",
  accepted: "neutral",
};

export function FindingStatusBadge({ status }: { status: FindingStatus }) {
  return <Badge tone={findingTone[status]}>{findingStatusLabel[status]}</Badge>;
}
