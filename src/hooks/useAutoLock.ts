import { useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { SessionInfo } from "@/types";

const TOUCH_INTERVAL_MS = 60_000;
const CHECK_INTERVAL_MS = 15_000;

export function useAutoLock(enabled: boolean, autoLockSeconds: number) {
  const queryClient = useQueryClient();
  const lastActivity = useRef(0);
  const lastTouch = useRef(0);
  const locking = useRef(false);

  useEffect(() => {
    if (!enabled) return;
    lastActivity.current = Date.now();

    const onActivity = () => {
      lastActivity.current = Date.now();
      if (Date.now() - lastTouch.current < TOUCH_INTERVAL_MS) return;
      lastTouch.current = Date.now();
      api.touchSession().catch(() => {
        void queryClient.invalidateQueries({ queryKey: ["session"] });
      });
    };

    const events: (keyof WindowEventMap)[] = ["pointerdown", "keydown", "wheel", "touchstart"];
    for (const event of events) {
      window.addEventListener(event, onActivity, { passive: true });
    }

    const timer = window.setInterval(() => {
      if (locking.current) return;
      if (Date.now() - lastActivity.current < autoLockSeconds * 1000) return;
      locking.current = true;
      api
        .lockVault()
        .then((info) => {
          queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== "session" });
          queryClient.setQueryData<SessionInfo>(["session"], info);
        })
        .catch(() => {
          void queryClient.invalidateQueries({ queryKey: ["session"] });
        })
        .finally(() => {
          locking.current = false;
        });
    }, CHECK_INTERVAL_MS);

    return () => {
      for (const event of events) {
        window.removeEventListener(event, onActivity);
      }
      window.clearInterval(timer);
    };
  }, [enabled, autoLockSeconds, queryClient]);
}
