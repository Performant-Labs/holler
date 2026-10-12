# Evidence: #808, a Herdr snapshot pane carries its label

Source facts the diff relies on that live in unchanged code. Each excerpt is copied from the branch after F (line numbers
are the post-F file's; the excerpted lines themselves are unchanged by the diff).

## F (Phase 6)

- **Fact:** the conformance runner drops each case's subject before its guard, so the label runner, which packs
  `(fixture, guard)` into the subject and passes `()` as the guard, still drops the fixture before its guard (a tuple
  drops its fields in declaration order).
  **Source:** `crates/holler-pane-testkit/src/conformance/mod.rs:55-58`
  **Verbatim excerpt:**
  > ```rust
  >             let (subject, guard) = fresh();
  >             let outcome = check(run, &subject);
  >             drop(subject);
  >             drop(guard);
  > ```

- **Fact:** `FakeHerdr`'s `Workspace::place` returns the id of the pane at `grid`: the occupant's, or one minted there,
  so `ensure_pane`'s lookup of that id among the snapshot's entries finds the pane at `spec.grid`.
  **Source:** `crates/holler-pane-testkit/src/herdr.rs:415-424`
  **Verbatim excerpt:**
  > ```rust
  >     fn place(&mut self, grid: GridPos, placement: Placement) -> Result<PaneId, PaneError> {
  >         self.check_range(grid)?;
  >         if let Some(occupant) = self.occupant(grid) {
  >             return Ok(occupant.clone());
  >         }
  >         if placement == Placement::SplitOnly {
  >             self.check_split(grid)?;
  >         }
  >         self.mint(grid)
  >     }
  > ```

- **Fact:** `FakeHerdr::ensure_pane` reaches `place` only when `spec.session` is the fake's session and the workspace's
  name is `spec.workspace`, so the snapshot entry it now answers carries the same session, workspace and cell that the
  old literal copied from `spec`.
  **Source:** `crates/holler-pane-testkit/src/herdr.rs:374-378`
  **Verbatim excerpt:**
  > ```rust
  >         let served = spec.session == self.session;
  >         let session = &self.session;
  >         self.workspaces
  >             .iter_mut()
  >             .find(|workspace| served && workspace.name == spec.workspace)
  > ```

- **Fact:** `FakeHerdr::rename` goes through `pane_mut`, which is `pane-not-found` for an id no workspace holds (a pane
  never minted, closed or vanished): the hook's error contract.
  **Source:** `crates/holler-pane-testkit/src/herdr.rs:390-396`
  **Verbatim excerpt:**
  > ```rust
  >     fn pane_mut(&mut self, id: &PaneId) -> Result<&mut LivePane, PaneError> {
  >         self.workspaces
  >             .iter_mut()
  >             .flat_map(|workspace| workspace.panes.values_mut())
  >             .find(|pane| pane.id == *id)
  >             .ok_or_else(|| not_found(id))
  >     }
  > ```

- **Fact:** O2 compares the `ensure_pane` answer, and the record is written from that same answer: `bring_up` takes
  `herdr` from `ensure_pane`, hands `&herdr` to `observe_live` (O1, O2) and returns it as `Live.herdr`, which
  `Plan::record` writes with its label cleared.
  **Source:** `crates/holler-pane/src/tx_launch.rs:620` and `crates/holler-pane/src/tx_launch.rs:645-648`
  **Verbatim excerpt:**
  > ```rust
  >     let herdr = ports.herdr.ensure_pane(&plan.target)?;
  > ```
  > ```rust
  >     let shown = observe_live(ports, plan.name, budget, &herdr, &session)?;
  >     budget.check()?;
  >     Ok(Live {
  >         herdr,
  > ```

## T (Phase 7, verify)

- **Fact:** reconcile's `herdr-pane-missing` check keys a live Herdr pane by `(session, pane_id)` only, so a label on a
  live pane cannot change whether a record's pane is found (the doctor guard `labels_change_nothing_doctor_reports`
  relies on it).
  **Source:** `crates/holler-pane/src/reconcile.rs:328-335`
  **Verbatim excerpt:**
  > ```rust
  >     let live: BTreeSet<(&str, &PaneId)> = snapshot
  >         .panes
  >         .iter()
  >         .map(|herdr| (herdr.session.as_str(), &herdr.pane_id))
  >         .collect();
  >     scope
  >         .iter()
  >         .filter(|pane| !live.contains(&(pane.herdr.session.as_str(), &pane.herdr.pane_id)))
  > ```

- **Fact:** reconcile's `unregistered-herdr-pane` check keys by `(session, pane_id)` too, and a finding carries only the
  Herdr pane's id and cell, never the whole `HerdrPane`, so a labelled world's `Report` and `pane doctor` output are
  identical to an unlabelled one's (same guard).
  **Source:** `crates/holler-pane/src/reconcile.rs:351-358` and `crates/holler-pane/src/findings.rs:270-276`
  **Verbatim excerpt:**
  > ```rust
  >     let named: BTreeSet<(&str, &PaneId)> = records
  >         .iter()
  >         .map(|pane| (pane.herdr.session.as_str(), &pane.herdr.pane_id))
  >         .collect();
  >     snapshot
  >         .panes
  >         .iter()
  >         .filter(|herdr| !named.contains(&(herdr.session.as_str(), &herdr.pane_id)))
  > ```
  > ```rust
  >     pub(crate) fn with_herdr_pane(self, herdr: &HerdrPane) -> Self {
  >         Self {
  >             herdr_pane: Some(herdr.pane_id.clone()),
  >             grid: Some(herdr.grid),
  >             ..self
  >         }
  >     }
  > ```
