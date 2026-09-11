# Official skills and MCP servers for the dz-pos stack (2026-09-08)

Nothing here was installed on 2026-09-08. Installing into Claude Code is
Samir's call; each row has the command. Checked again 2026-09-11: `skill-creator`
was already a plugin at the time this page was written (noted in its own row
below); `frontend-design` has since been installed too, see its row.

## Anthropic (`anthropics/skills`, plugin marketplace `anthropic-agent-skills`)

Skills in the repo on 2026-09-08: academy-guide, algorithmic-art,
brand-guidelines, canvas-design, claude-api, discernment-nudge,
doc-coauthoring, docx, frontend-design, internal-comms, mcp-builder, pdf,
pptx, skill-creator, slack-gif-creator, theme-factory, web-artifacts-builder,
webapp-testing, xlsx.

| Skill | Use for dz-pos | Verdict |
|---|---|---|
| `frontend-design` | the Tauri screens once real code replaces the mockups | try on the first real screen; compare with the mockup's tokens. Installed 2026-09-11 (`~/.claude/plugins/installed_plugins.json`), currently disabled in `~/.claude/settings.json`; the trial itself is still R9, unrecorded |
| `webapp-testing` | Playwright discipline for the desktop webview and the mockups | yes; our `dz-mockup` drive scripts do this by hand today |
| `pdf` | reading the DGI code PDFs (`legal-fiscal/sources/`) | useful for research sessions |
| `mcp-builder` | only if we expose the localhost API to agents later | not now |
| `skill-creator` | already installed as a plugin | — |

Install: `/plugin marketplace add anthropics/skills` then
`/plugin install example-skills@anthropic-agent-skills` (frontend-design,
webapp-testing, mcp-builder live there) or `document-skills@...` (pdf, docx,
xlsx, pptx).

## Expo (official, `expo/skills`, plugin `expo@claude-plugins-official`)

24 skills on 2026-09-08: eas-app-stores, eas-hosting, eas-observe,
eas-simulator, eas-update, eas-update-insights, eas-workflows,
expo-animation, expo-app-clip, expo-brownfield, expo-data-fetching,
expo-design-system, expo-dev-client, expo-dom, expo-examples, expo-module,
expo-native-ui, expo-overview, expo-project-structure, expo-router,
expo-skill-feedback, expo-ui, expo-upgrade, expo-web-to-native.

Relevant when `apps/mobile` starts: `expo-project-structure`, `expo-router`,
`expo-design-system` (matches the tokens approach from the mockups),
`expo-data-fetching`, `expo-native-ui`, `expo-module` (if the Rust core is
ever bound with uniffi), `eas-workflows`. The Expo MCP server (remote,
hosted by Expo; docs.expo.dev/mcp) gives SDK docs, simulator and DevTools
access. Install both with `claude plugin install expo@claude-plugins-official`.
Verdict: install on the laptop when mobile work begins, not before.

## Tauri

No official Tauri skill or MCP from the Tauri project. Community options:

| Project | What | Notes |
|---|---|---|
| hypothesi `mcp-server-tauri` (+ `@hypothesi/tauri-mcp-cli`, ships agent skills) | screenshots, DOM, console, IPC monitoring, webview automation against a running Tauri v2 app | needs `tauri-plugin-mcp-bridge` compiled into debug builds and a running app with a display, so laptop only. Unofficial, states so. |
| P3GLEG `tauri-plugin-mcp` | similar, plugin-side | smaller |
| EpicenterHQ `tauri` skill (mcp.directory/skills/tauri) | prose skill, no runtime | cheap to read |

Verdict: evaluate hypothesi on the laptop once the first real screen
exists; our architecture routes the webview through the localhost HTTP API,
so Playwright against `pnpm desktop dev` covers most of what the MCP bridge
would, without a plugin in the binary. Decide after one trial.

## Rust side

No skills needed today. Docs with `llms.txt` for agents: axum
(docs.rs), diesel guides (diesel.rs/guides), Tauri v2 (v2.tauri.app, has
`/llms.txt`). Add them to the tooling page when a session actually uses
one.
