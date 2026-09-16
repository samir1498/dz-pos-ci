// The theme.

import { createFileRoute } from "@tanstack/react-router";

import { ThemePanel } from "@/components/settings/ThemePanel";

export const Route = createFileRoute("/settings/appearance")({ component: ThemePanel });
