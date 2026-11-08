import type { ReactNode } from "react";
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import { cn } from "@/lib/cn";

export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  footer,
  wide,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  children: ReactNode;
  footer?: ReactNode;
  wide?: boolean;
}) {
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-slate-950/80" />
        <DialogPrimitive.Content
          className={cn(
            "fixed left-1/2 top-1/2 z-50 max-h-[88vh] w-[calc(100vw-2rem)] -translate-x-1/2 -translate-y-1/2 overflow-hidden rounded-xl border border-slate-800 bg-slate-900 shadow-2xl shadow-black/60",
            wide ? "max-w-4xl" : "max-w-xl",
          )}
        >
          <div className="flex items-start justify-between gap-4 border-b border-slate-800 px-5 py-4">
            <div>
              <DialogPrimitive.Title className="text-base font-semibold text-white">
                {title}
              </DialogPrimitive.Title>
              {description && (
                <DialogPrimitive.Description className="mt-1 text-xs text-slate-400">
                  {description}
                </DialogPrimitive.Description>
              )}
            </div>
            <DialogPrimitive.Close
              className="rounded-md p-1 text-slate-400 transition-colors hover:bg-slate-800 hover:text-white"
              aria-label="Close"
            >
              <X className="h-4 w-4" />
            </DialogPrimitive.Close>
          </div>
          <div className="max-h-[calc(88vh-8.5rem)] overflow-y-auto px-5 py-4">{children}</div>
          {footer && (
            <div className="flex items-center justify-end gap-2 border-t border-slate-800 bg-slate-950/40 px-5 py-3">
              {footer}
            </div>
          )}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
