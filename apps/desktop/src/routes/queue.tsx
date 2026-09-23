// The clinic's waiting queue (C6). Thin route file for the same reason
// `patients.tsx` beside it is: see that file's header.

import { createFileRoute } from "@tanstack/react-router";

import { QueueScreen } from "./-queue/QueueScreen";

export const Route = createFileRoute("/queue")({ component: QueueScreen });
