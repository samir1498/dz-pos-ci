// TanStack Query, configured for a till on a shop's Wi-Fi rather than for a
// page on a desk.
//
// The two settings that matter here are the ones about failure. A 4xx is the
// server having read the request and refused it — retrying cannot change the
// answer, and the phone spent this morning proving what happens when it
// tries anyway. Anything else is worth one retry, because a shop's Wi-Fi
// drops a packet far more often than a desk's does.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState, type ReactNode } from "react";

import { ApiRefusal } from "../lib/api";

export function QueryProvider({ children }: { children: ReactNode }) {
  const [client] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 5 * 60 * 1000,
            gcTime: 10 * 60 * 1000,
            retry: (failures, error) => {
              if (error instanceof ApiRefusal) return false;
              return failures < 1;
            },
            refetchOnWindowFocus: false,
          },
          // A sale is never retried by the query layer: the till owns that,
          // because a retry has to carry the same idempotency key as the
          // first try and only the till knows which key that was.
          mutations: { retry: false },
        },
      }),
  );
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
