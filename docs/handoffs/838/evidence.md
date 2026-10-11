# Evidence: #838 herdr-session-question

- **Fact:** `instance.sh` refuses a non-default instance named `default`, so the named branch says the name must not be `default`.
  **Source:** `agent-skills/herdr-setup/lib/instance.sh:154-155`
  **Verbatim excerpt:**
  > if (nondefault) {
  >     if (!("name" in set) || name == "default") err("[instance] a non-default instance must set name (to something other than default)")

- **Fact:** the outside-a-pane rule the new question points to ("rule (2)") is already stated in the unchanged "Two rules for a non-default instance" paragraph.
  **Source:** `agent-skills/herdr-setup/SKILL.md:704-707`
  **Verbatim excerpt:**
  > session sets its own `backend_port` (exit 1, one line naming the key). (2) It is driven from a
  > terminal outside any Herdr pane: a driving agent that runs inside a Herdr pane cannot establish
  > which session it is in, and Stage 2's `check-pane` stops it (the fix is to start the wizard from a
  > plain terminal, or from the desktop app, not from a pane).
