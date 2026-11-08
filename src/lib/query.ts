import { QueryClient } from "@tanstack/react-query";
import { IpcError } from "@/lib/ipc";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: (failureCount, error) => {
        if (error instanceof IpcError && /locked|expired/i.test(error.message)) return false;
        return failureCount < 1;
      },
      refetchOnWindowFocus: false,
      staleTime: 5_000,
    },
    mutations: {
      retry: false,
    },
  },
});
