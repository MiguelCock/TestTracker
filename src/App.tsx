import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { QueryClientProvider } from "@tanstack/react-query";
import { ShieldCheck } from "lucide-react";
import { AppShell } from "@/components/layout/AppShell";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/states";
import { useSession } from "@/hooks/useSession";
import { queryClient } from "@/lib/query";
import { UnlockScreen } from "@/features/auth/UnlockScreen";
import { DashboardPage } from "@/features/dashboard/DashboardPage";
import { EngagementDetailPage } from "@/features/engagements/EngagementDetailPage";
import { EngagementListPage } from "@/features/engagements/EngagementListPage";
import { McpPage } from "@/features/mcp/McpPage";
import { SettingsPage } from "@/features/settings/SettingsPage";

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Root />
    </QueryClientProvider>
  );
}

function Root() {
  const session = useSession();

  if (session.isLoading && !session.data) return <Splash />;

  if (!session.data) {
    if (!session.isError) return <Splash />;
    return (
      <div className="flex min-h-full items-center justify-center p-6">
        <div className="w-full max-w-md rounded-xl border border-red-500/30 bg-slate-900 p-6 text-center">
          <p className="text-sm font-medium text-red-300">Unable to open TestTracker</p>
          <p className="mt-2 text-xs text-slate-500">{session.error.message}</p>
          <Button className="mt-4" variant="secondary" onClick={() => session.refetch()}>
            Try again
          </Button>
        </div>
      </div>
    );
  }

  if (!session.data.initialized || !session.data.unlocked) {
    return <UnlockScreen initialized={session.data.initialized} />;
  }

  return (
    <HashRouter>
      <Routes>
        <Route element={<AppShell session={session.data} />}>
          <Route index element={<DashboardPage />} />
          <Route path="engagements" element={<EngagementListPage />} />
          <Route path="engagements/:id" element={<EngagementDetailPage />} />
          <Route path="mcp" element={<McpPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}

function Splash() {
  return (
    <div className="flex min-h-full flex-col items-center justify-center gap-4">
      <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-emerald-500/15 ring-1 ring-emerald-500/30">
        <ShieldCheck className="h-6 w-6 text-emerald-400" />
      </div>
      <div className="flex items-center gap-2 text-xs text-slate-500">
        <Spinner /> Opening secure workspace
      </div>
    </div>
  );
}
