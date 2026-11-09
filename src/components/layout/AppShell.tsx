import { NavLink, Outlet } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { FolderKanban, LayoutDashboard, Lock, Network, Settings, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useAutoLock } from "@/hooks/useAutoLock";
import { useLockVault } from "@/hooks/useSession";
import { api } from "@/lib/api";
import { cn } from "@/lib/cn";
import { initials } from "@/lib/format";
import { roleLabel } from "@/lib/labels";
import type { SessionInfo } from "@/types";

const navItems = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard, end: true },
  { to: "/engagements", label: "Engagements", icon: FolderKanban, end: false },
  { to: "/mcp", label: "MCP", icon: Network, end: false },
  { to: "/settings", label: "Settings", icon: Settings, end: false },
];

export function AppShell({ session }: { session: SessionInfo }) {
  useAutoLock(true, session.autoLockSeconds);
  const lock = useLockVault();
  const mcp = useQuery({
    queryKey: ["mcp-status"],
    queryFn: api.mcpStatus,
    refetchInterval: 5_000,
    retry: false,
  });
  const profile = session.profile;
  const mcpPending = mcp.data?.running ? mcp.data.pendingCount : 0;

  return (
    <div className="flex min-h-full bg-slate-950">
      <aside className="flex w-64 shrink-0 flex-col border-r border-slate-800 bg-slate-950/80">
        <div className="flex items-center gap-3 px-5 py-5">
          <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-emerald-500/15 ring-1 ring-emerald-500/30">
            <ShieldCheck className="h-4.5 w-4.5 text-emerald-400" />
          </div>
          <div>
            <p className="text-sm font-semibold tracking-wide text-white">TestTracker</p>
            <p className="text-[10px] uppercase tracking-[0.18em] text-slate-500">
              Local encrypted vault
            </p>
          </div>
        </div>

        <nav className="flex-1 space-y-1 px-3 py-2">
          {navItems.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                cn(
                  "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                  isActive
                    ? "bg-slate-800/80 text-white"
                    : "text-slate-400 hover:bg-slate-900 hover:text-slate-200",
                )
              }
            >
              <Icon className="h-4 w-4" />
              {label}
              {to === "/mcp" && mcpPending > 0 && (
                <span className="ml-auto rounded-full bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-semibold text-amber-300">
                  {mcpPending}
                </span>
              )}
            </NavLink>
          ))}
        </nav>

        <div className="mx-3 mb-3 rounded-lg border border-emerald-500/15 bg-emerald-500/5 p-3">
          <div className="flex items-center gap-2 text-[11px] font-medium text-emerald-300">
            <Lock className="h-3.5 w-3.5" />
            Vault unlocked
          </div>
          <p className="mt-1 text-[11px] leading-relaxed text-slate-500">
            Auto-locks after {Math.round(session.autoLockSeconds / 60)} minutes of inactivity.
          </p>
        </div>

        <div className="border-t border-slate-800 p-3">
          <div className="flex items-center gap-3 rounded-lg px-2 py-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-full bg-slate-800 text-xs font-semibold text-slate-200">
              {profile ? initials(profile.displayName) : "?"}
            </div>
            <div className="min-w-0 flex-1">
              <p className="truncate text-xs font-medium text-slate-200">
                {profile?.displayName ?? "Unknown"}
              </p>
              <p className="text-[10px] text-slate-500">{profile ? roleLabel[profile.role] : ""}</p>
            </div>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Lock vault"
              title="Lock vault"
              onClick={() => lock.mutate()}
              disabled={lock.isPending}
            >
              <Lock className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </aside>

      <main className="min-h-screen flex-1 overflow-y-auto">
        <div className="mx-auto max-w-7xl px-6 py-6 lg:px-8 lg:py-8">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
