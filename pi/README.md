# Pi distribution

This workspace contains the TypeScript extensions that dotfiles deploys for
vanilla Pi 0.84.4. Pi loads the source through jiti; there is no build or
`dist/` step.

Node 22.19 or newer, pnpm 11.18.0, and the Pi CLI are machine prerequisites.
Dotfiles does not install those tools. A real `dots sync` runs the workspace's
frozen pnpm install before planning the managed links, then deploys:

- `src/distribution/` to `~/.pi/agent/extensions/pi-distribution`;
- `src/subagent-workflow/` to `~/.pi/agent/extensions/subagent-workflow`;
- the `workflow-authoring` skill to `~/.pi/agent/skills/`.

The workflow runtime resolves Acorn from this workspace's `node_modules` through
the extension source realpath. Pi supplies its own extension packages and
TypeBox at runtime. Their exact development versions remain installed here for
the single source typecheck.

## Develop and verify

```sh
cd ~/dotfiles/pi
pnpm install --frozen-lockfile
pnpm typecheck
```

Normal deployment needs only:

```sh
cd ~/dotfiles
./dots.sh sync
```

`dots sync --dry-run` displays the dependency hook but does not run it.
`dots status` checks only Resource convergence and never runs lifecycle hooks.

## Workflow source ownership

`src/subagent-workflow/` is maintained directly as part of this Distribution.
There is no upstream refresh command, provenance manifest, or patch stack. Its
`LICENSE` retains Matt Zenko's complete MIT notice. Acorn is installed from the
exact pnpm lockfile; TypeBox and the Pi packages are supplied by Pi at runtime.

## Local state boundary

Dotfiles owns Pi settings, `AGENTS.md`, shared hook rules, extensions, and the
workflow-authoring skill. Pi continues to own `auth.json`, sessions, trust,
model storage, workflow journals, and all other machine-local runtime state.
