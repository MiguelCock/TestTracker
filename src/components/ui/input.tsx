import type {
  InputHTMLAttributes,
  LabelHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from "react";
import { cn } from "@/lib/cn";

const control =
  "w-full rounded-lg border border-slate-700 bg-slate-950/60 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 transition-colors hover:border-slate-600 focus:border-emerald-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60";

export function Input({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={cn(control, "h-10", className)} {...props} />;
}

export function Textarea({ className, ...props }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea className={cn(control, "min-h-24 resize-y", className)} {...props} />;
}

export function Select({ className, ...props }: SelectHTMLAttributes<HTMLSelectElement>) {
  return <select className={cn(control, "h-10 pr-8", className)} {...props} />;
}

export function Label({ className, ...props }: LabelHTMLAttributes<HTMLLabelElement>) {
  return (
    <label
      className={cn(
        "mb-1.5 block text-xs font-medium uppercase tracking-wide text-slate-400",
        className,
      )}
      {...props}
    />
  );
}

export function Field({
  label,
  hint,
  error,
  children,
  className,
}: {
  label: string;
  hint?: string;
  error?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("space-y-1", className)}>
      <Label>{label}</Label>
      {children}
      {hint && !error && <p className="text-xs text-slate-500">{hint}</p>}
      {error && <p className="text-xs text-red-400">{error}</p>}
    </div>
  );
}
