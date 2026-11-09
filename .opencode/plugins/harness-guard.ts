// Stateless guard. Rejections only; no state, no events, no timers.
//   1. subagent: no `model` argument (it would override the model pinned for the agent)            model drift (R3 A.2)
//   2. shell: no trailing `&` (use the shell tool's `background: true`)                            V-F04
//   3. shell: no banned command behind a prefix (`FOO=1 sleep`, `env timeout`, `sudo nohup`, ...)   V-F04
//      Each piece of a compound command (split at && || ; | newline) is checked at its start. `bash -c "sleep 1"` and `python -c` are not caught (audit-session.sh counts them).

const BACKGROUND_AMPERSAND = /(^|[^&>|\\])&[ \t]*$/
const BANNED = "(sleep|timeout|nohup|disown|systemd-run)"
const PREFIXED = new RegExp(`^\\s*((\\w+=\\S*|env|command|exec|sudo|xargs|nice|setsid)\\s+(-\\S+\\s+)*)+${BANNED}(\\s|$)`)
const PIECES = /&&|\|\||[;|\n]/

export default {
  id: "harness-guard",
  async setup(ctx) {
    await ctx.tool.hook("execute.before", (event) => {
      const input = event.input
      if (event.tool === "subagent" && input && typeof input.model === "string" && input.model !== "") {
        throw new Error("Rejected: do not pass `model` to subagent. The model is fixed by the agent definition in opencode.jsonc.")
      }
      if (event.tool !== "shell" || !input || typeof input.command !== "string") return
      const command = input.command.trimEnd()
      if (BACKGROUND_AMPERSAND.test(command)) {
        throw new Error("Rejected: a trailing `&` is not allowed. Use the shell tool with background: true.")
      }
      if (command.split(PIECES).some((piece) => PREFIXED.test(piece))) {
        throw new Error("Rejected: sleep, timeout, nohup, disown and systemd-run are banned, also behind a prefix such as env or VAR=value.")
      }
    })
  },
}
