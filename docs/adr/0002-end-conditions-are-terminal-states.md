# End Conditions are Terminal States

Videos that stopped at a hidden threshold (a time cap, a ball count, a max speed, a minimum radius) ended out of nowhere, while closing_in, which ends when the ball fills the shrinking Arena, felt earned. So every End Condition must reach a Terminal State: one the Scenario cannot carry on from and a viewer recognises without reading a number. Scenario validation rejects anything else, and the time cap is only a safety net whose Seeds the Quality Gate rejects. We deleted `time_cap`, `population_cap`, `max_speed_reached`, `min_radius_reached` and `arena_min_reached` as choosable End Conditions, and gave each Scenario a terminal ending (filled, vanished, escaped, covered, Meltdown) with an optional Play-out before the freeze.

## Considered Options

- **Keep thresholds and document a convention**: cheaper, but nothing stops the next Scenario ending at an arbitrary number again.
- **Mark End Conditions terminal or threshold and refuse the latter**: keeps dead code around; deleting them leaves one kind of End Condition.

## Consequences

- The Meltdown ("until it breaks") is staged, not real. Because rendering is offline and deterministic (ADR 0001), the simulation struggling never shows in the Video. So the Meltdown fires deterministically at the Scenario's Population Cap, and its Play-out fakes the breakdown (stutter, glitch, audio crackle, hard cut) with no flash or final chord.
- `arena_covered` is a threshold underneath (a fixed ~98% coverage). It counts as terminal only because at that level the Arena visibly reads as full, so Scenarios cannot set the fraction.
