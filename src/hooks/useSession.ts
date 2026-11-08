import { useEffect } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { SessionInfo } from "@/types";

export function useSession() {
  const queryClient = useQueryClient();
  useEffect(() => {
    const handler = () => {
      void queryClient.invalidateQueries({ queryKey: ["session"] });
    };
    window.addEventListener("testtracker:session-locked", handler);
    return () => window.removeEventListener("testtracker:session-locked", handler);
  }, [queryClient]);
  return useQuery({
    queryKey: ["session"],
    queryFn: api.sessionInfo,
    refetchInterval: 30_000,
    staleTime: 0,
    retry: false,
  });
}

export function useLockVault() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async () => {
      const info = await api.lockVault();
      queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== "session" });
      queryClient.setQueryData<SessionInfo>(["session"], info);
      return info;
    },
  });
}

export function useUnlockVault() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (passphrase: string) => api.unlockVault(passphrase),
    onSuccess: (info) => {
      queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== "session" });
      queryClient.setQueryData<SessionInfo>(["session"], info);
    },
  });
}
