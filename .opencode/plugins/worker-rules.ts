// Stateless. A new worker/reviewer session must start with the worker rules (V-F06: 28 of 99 starts after the rule was made left them out).
// If the prompt does not start with the marker, the text of prompts/worker-rules.md is put in front of it.
// The text is read at call time; nothing is embedded here. If it cannot be read, the call fails and says why (never passes silently).
// Follow-ups to an existing session (sessionID given) are left alone: that session already has the rules.
import { readFile } from "node:fs/promises"

const MARKER = "<!-- worker-rules v1 -->"

export default {
  id: "worker-rules",
  async setup(ctx) {
    await ctx.tool.hook("execute.before", async (event) => {
      const input = event.input
      if (event.tool !== "subagent" || !input || input.sessionID) return
      if (input.agent !== "worker" && input.agent !== "reviewer") return
      const prompt = typeof input.prompt === "string" ? input.prompt : ""
      if (prompt.trimStart().startsWith(MARKER)) return
      const file = ctx.location.directory + "/prompts/worker-rules.md"
      const rules = await readFile(file, "utf8").catch((error) => {
        throw new Error("Rejected: cannot read " + file + " (" + (error && error.code) + "). Restore the file; it is put in front of every worker/reviewer prompt.")
      })
      if (!rules.startsWith(MARKER)) throw new Error("Rejected: " + file + " must start with the line " + MARKER)
      event.input = { ...input, prompt: rules.trimEnd() + "\n\n" + prompt }
    })
  },
}
