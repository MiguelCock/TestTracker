import type { ReactNode } from "react";
import * as TabsPrimitive from "@radix-ui/react-tabs";
import { cn } from "@/lib/cn";

export const Tabs = TabsPrimitive.Root;

export function TabsList({ className, ...props }: TabsPrimitive.TabsListProps) {
  return (
    <TabsPrimitive.List
      className={cn(
        "inline-flex items-center gap-1 rounded-lg border border-slate-800 bg-slate-900/60 p-1",
        className,
      )}
      {...props}
    />
  );
}

export function TabsTrigger({ className, ...props }: TabsPrimitive.TabsTriggerProps) {
  return (
    <TabsPrimitive.Trigger
      className={cn(
        "rounded-md px-3 py-1.5 text-xs font-medium text-slate-400 transition-colors hover:text-slate-200 data-[state=active]:bg-slate-800 data-[state=active]:text-white",
        className,
      )}
      {...props}
    />
  );
}

export function TabsContent({ className, children, ...props }: TabsPrimitive.TabsContentProps) {
  return (
    <TabsPrimitive.Content className={cn("mt-4 focus:outline-none", className)} {...props}>
      {children}
    </TabsPrimitive.Content>
  );
}

export function TabPanel({ value, children }: { value: string; children: ReactNode }) {
  return <TabsContent value={value}>{children}</TabsContent>;
}
