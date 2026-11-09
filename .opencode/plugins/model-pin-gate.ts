// Stateless gate. A tool call is rejected when its session runs a model other than the one pinned for its agent.   model drift (R3 A.2, B.7)
// The pin is read from the agent definition (opencode.jsonc), so scripts/set-model-profile.sh needs no change here.
// Limits: only tool calls are stopped (a text-only reply still runs), and the check compares provider and model, not the variant.
// Not verified in a real session when this was written (only against mocks); see tests/harness/README.md for the canary.
const GATED = ["build", "worker", "reviewer"]

export default {
  id: "model-pin-gate",
  async setup(ctx) {
    await ctx.tool.hook("execute.before", async (event) => {
      if (!GATED.includes(event.agent)) return
      const agent = await ctx.agent.get({ agentID: event.agent })
      const pin = agent && agent.data && agent.data.model
      if (!pin) throw new Error("Rejected: agent " + event.agent + " has no model in opencode.jsonc; the configuration may not be loaded.")
      const session = await ctx.session.get({ sessionID: event.sessionID })
      const model = session.model
      if (!model) return
      if (model.providerID === pin.providerID && model.id === pin.id) return
      throw new Error(
        "Rejected: session model " + model.providerID + "/" + model.id + " is not the model " + pin.providerID + "/" + pin.id +
          " pinned for agent " + event.agent + ". Tool calls stay blocked until the session's model is switched back. Write one line 'model switch detected' in the report.",
      )
    })
  },
}
